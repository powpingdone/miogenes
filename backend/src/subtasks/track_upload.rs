use axum::http::StatusCode;
use glib::SendValue;
use gstreamer::glib;
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use log::*;
use path_absolutize::Absolutize;
use sha2::{
    Digest,
    Sha256,
};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use uuid::*;
use crate::*;

// metadata parsed from the individual file
#[derive(Default)]
struct Metadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    overflow: Option<String>,
    img: Option<Vec<u8>>,
    imghash: Option<[u8; 32]>,
    audiohash: Option<[u8; 32]>,
    disk_track: (Option<i32>, Option<i32>),
}

// TODO: upload process time limits
//
// TODO: size limits
pub async fn track_upload_process(
    state: MioState,
    id: Uuid,
    userid: Uuid,
    orig_filename: String,
) -> Result<(), StatusCode> {
    // process metadata
    let permit = state.lim.acquire().await.unwrap();
    let mdata = tokio::task::spawn_blocking({
        let orig_filename = orig_filename.clone();
        move || {
            get_metadata(format!("{}{}", DATA_DIR.get().unwrap(), id), orig_filename)
        }
    }).await.expect("join failure").map_err(|err| {
        Into::<StatusCode>::into(MioInnerError::TrackProcessingError(err, StatusCode::BAD_REQUEST))
    })?;
    drop(permit);

    // insert into the database
    insert_into_db(state.db, id, userid, mdata, orig_filename).await.map_err(tr_conv_code)
}

fn get_metadata(fname: String, orig_path: String) -> Result<Metadata, anyhow::Error> {
    // TODO: make this timeout configurable
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(10))?;
    trace!("{orig_path}: creating discoverer");
    let fname = glib::filename_to_uri(Path::new(&fname).absolutize()?, None)?;
    trace!("{orig_path}: new uri created: '{fname}'");
    debug!("{orig_path}: begin discovery");
    let data = discover.discover_uri(&fname)?;

    // TODO: implement errors for the rest (possibly, may not be needed)
    trace!("{orig_path}: result: {:?}", data.result());
    match data.result() {
        DiscovererResult::Ok => (),
        DiscovererResult::MissingPlugins => {
            anyhow::bail!("Missing plugin needed for file {orig_path}");
        },
        DiscovererResult::Timeout => {
            anyhow::bail!("Timeout reached for processing tags")
        },
        // these branches _shouldn't_ fail.
        //
        // Busy -> each discoverer is in it's own thread, where it only reads one file
        DiscovererResult::Busy => unimplemented!(),
        // UriInvalid -> fname is produced via glib::filename_to_uri
        DiscovererResult::UriInvalid => unimplemented!(),
        // Error -> discover_uri can return an error, so this shouldn't happen here
        DiscovererResult::Error => unimplemented!(),
        _other => panic!("unhandled enum: {_other:?}"),
    }

    // tag metadata
    debug!("{orig_path}: collecting tags");
    let mut mdata: Metadata = Default::default();

    // iterate through all tags
    let mut set = HashMap::new();
    for (tag, data) in data.audio_streams().iter().flat_map(|streaminfo| {
        let tags = streaminfo.tags();
        let mut ret = vec![];
        if let Some(tags) = tags {
            for (tag, value) in tags.iter() {
                trace!("{orig_path}: tag proc'd \"{tag}\"");
                ret.push((tag.to_owned(), value));
            }
        } else {
            debug!("{orig_path}: streaminfo.tags() produced a none");
        };
        ret
    }) {
        trace!("{orig_path}: tag \"{tag}\"");
        match tag.as_str() {
            // TODO: verify this is the right thing
            "image" => {
                let samp = data.get::<gstreamer::Sample>()?;
                if let Some(bufs) = samp.buffer() {
                    let img: Vec<u8> = {
                        let mut imgbuf: Vec<u8> = vec![];
                        for buf in bufs {
                            // TODO: make this error better
                            buf.map_readable().expect("memory should be readable: {}").iter().for_each(|x| imgbuf.push(*x));
                        }
                        imgbuf
                    };
                    mdata.imghash = Some(hash(&img));
                    trace!("{orig_path}: imghash: {:?}", mdata.imghash);

                    // convert to webp
                    let dropped = image::load_from_memory(&img)?;
                    let conv = dropped.into_rgb8();
                    let mut hold = Cursor::new(vec![]);
                    conv.write_to(&mut hold, image::ImageFormat::WebP)?;
                    mdata.img = Some(hold.into_inner());
                } else {
                    anyhow::bail!("no buffer found for image");
                }
            },
            "title" => {
                mdata.title = proc_tag(data);
                trace!("{orig_path}: title is {:?}", mdata.title)
            },
            "artist" => {
                mdata.artist = proc_tag(data);
                trace!("{orig_path}: artist is {:?}", mdata.artist)
            },
            "album" => {
                mdata.album = proc_tag(data);
                trace!("{orig_path}: album is {:?}", mdata.album)
            },
            "album-disc-number" | "track-number" => {
                let mut_info = if tag.as_str() == "album-disc-number" {
                    &mut mdata.disk_track.0
                } else {
                    &mut mdata.disk_track.1
                };
                *mut_info = proc_tag(data).and_then(|inp| {
                    match inp.parse() {
                        Ok(ok) => Some(ok),
                        Err(err) => {
                            debug!("{orig_path}: error parsing int out {err}");
                            None
                        },
                    }
                });
                trace!("{orig_path}: disk_track is {:?}", mdata.disk_track)
            },
            _ => {
                // generic handler
                let data = proc_tag(data);
                if data.is_some() {
                    let ret = set.insert(tag.clone(), data.clone());

                    fn truncate(x: String) -> String {
                        const LEN: usize = 80;
                        if x.len() > LEN {
                            let mut trunc = x.chars().take(LEN).collect::<String>();
                            trunc.push_str("...");
                            trunc
                        } else {
                            x
                        }
                    }

                    let (data, ret) = ({
                        truncate(format!("{data:?}"))
                    }, {
                        truncate(format!("{ret:?}"))
                    });
                    trace!("{orig_path}: KV inserted ({tag}, {data}), replaced ({tag}, {ret})");
                }
            },
        }
    }
    if !set.is_empty() {
        mdata.overflow =
            Some(
                serde_json::to_string(
                    &set.into_iter().map(|(a, b)| (a.to_string(), b)).collect::<HashMap<_, _>>(),
                )?,
            );
    }
    if mdata.title.is_none() {
        warn!("{orig_path}: this song has no \"title\" tag, using filename");
        mdata.title = Some(orig_path.to_owned())
    }
    drop(discover);

    // audiohash
    trace!("{orig_path}: beginning audiohash");
    let pipeline =
        gstreamer::parse_launch(&format!("uridecodebin3 uri={fname} ! audioconvert ! appsink name=sink"))?
            .downcast::<gstreamer::Pipeline>()
            .expect("Expected a gst::Pipeline");

    // sink extractor
    //
    // TODO: either do a shared memory thing or Oneshot it
    let (tx, rx) = std::sync::mpsc::channel();
    let sink =
        pipeline
            .by_name("sink")
            .expect("sink element not found")
            .dynamic_cast::<AppSink>()
            .expect("failed dynamic cast to AppSink");
    sink.set_property("sync", false);
    sink.set_callbacks({
        let orig_path = orig_path.to_owned();
        gstreamer_app::AppSinkCallbacks::builder().new_sample(move |sink| {
            trace!("{orig_path}: sink match from main loop entered");
            match sink.pull_sample() {
                Ok(sample) => match sample.buffer() {
                    Some(buflist) => {
                        trace!("{orig_path}: found some buffers, sending over");
                        tx.send(buflist.iter_memories().flat_map(|buf| {
                            buf
                                .map_readable()
                                .expect("memory should be readable")
                                .iter()
                                .copied()
                                .collect::<Vec<_>>()
                        }).collect::<Vec<_>>()).unwrap();
                        Err(gstreamer::FlowError::Eos)
                    },
                    None => {
                        debug!("{orig_path}: failed to get buffer list as none was produced");
                        Err(gstreamer::FlowError::Error)
                    },
                },
                Err(err) => {
                    debug!("{orig_path}: failed to grab sample: {err}");
                    Err(gstreamer::FlowError::Error)
                },
            }
        }).build()
    });

    // begin the actual fetching
    pipeline.set_state(gstreamer::State::Playing)?;
    debug!("{orig_path}: entering waveform capture loop");
    let bus = pipeline.bus().expect("pipeline without a bus");
    for msg in bus.iter_timed(gstreamer::ClockTime::NONE) {
        use gstreamer::MessageView;

        match msg.view() {
            MessageView::Eos(_) => break,
            MessageView::Error(err) => anyhow::bail!("failed to execute pipeline: {err:#?}"),
            _ => (),
        }
    }
    debug!("{orig_path}: exiting waveform capture loop");
    pipeline.set_state(gstreamer::State::Null)?;
    trace!("{orig_path}: collecting iter");
    let waveform = rx.recv().unwrap();
    trace!("{orig_path}: hashing");
    mdata.audiohash = Some(hash(&waveform));
    trace!("{orig_path}: audiohash is {:?}", mdata.audiohash);
    Ok(mdata)
}

// copy the hash of one value into a regular array since sha2 uses GenericArray's,
// this is done via just manually iterating through it
fn hash(data: &[u8]) -> [u8; 32] {
    let sha = Sha256::digest(data);
    let mut actual_hash: [u8; 32] = Default::default();
    for (hasharr, digested) in actual_hash.iter_mut().zip(sha.iter()) {
        *hasharr = *digested;
    }
    actual_hash
}

// TODO: log failures
fn proc_tag(data: SendValue) -> Option<String> {
    if let Ok(x) = data.get::<String>() {
        trace!("got string directly: {x}");
        Some(x)
    } else if let Ok(x) = data.serialize() {
        trace!("serialized string: {x}");
        Some(x.to_string())
    } else {
        trace!("no string created");
        None
    }
}

async fn insert_into_db(
    db: SqlitePool,
    id: Uuid,
    userid: Uuid,
    metadata: Metadata,
    orig_filename: String,
) -> Result<(), TransactionError<MioInnerError>> {
    db.transaction::<_, _, MioInnerError>(|txn| Box::pin(async move {
        // transfer ownership
        let hold = metadata;
        let metadata = &hold;

        // insert cover art, compare based on imghash
        debug!("{orig_filename}: Filling cover art");
        let (cover_art_new_row, cover_art_id) = create_model_and_id(txn, &metadata.imghash, || {
            cover_art::Column::ImgHash.eq(metadata.imghash.unwrap().to_vec())
        }, || {
            cover_art::ActiveModel {
                id: NotSet,
                webm_blob: Set(metadata.img.to_owned().unwrap()),
                img_hash: Set(metadata.imghash.unwrap().to_vec()),
            }
        }, cover_art::Column::Id).await?;
        if cover_art_new_row.is_some() {
            trace!("{orig_filename}: new cover art generated: {cover_art_id:?}");
        } else {
            trace!("{orig_filename}: cover art already exists: {cover_art_id:?}");
        }

        // insert artist, compare based on artist name
        debug!("{orig_filename}: Filling artist");
        let (artist_new_row, artist_id) = create_model_and_id(txn, &metadata.artist, || {
            artist::Column::Name.eq(metadata.artist.as_ref().unwrap())
        }, || {
            artist::ActiveModel {
                id: NotSet,
                name: Set(metadata.artist.to_owned().unwrap()),
                sort_name: Set(None),
            }
        }, artist::Column::Id).await?;
        if artist_new_row.is_some() {
            trace!("{orig_filename}: new artist generated: {artist_new_row:?}");
        } else {
            trace!("{orig_filename}: artist already exists: {artist_id:?}");
        }

        // insert album, compare based on album title
        //
        // TODO: also join based on album artist
        debug!("{orig_filename}: Filling album");
        let (album_new_row, album_id) = create_model_and_id(txn, &metadata.album, || {
            album::Column::Title.eq(metadata.album.as_ref().unwrap())
        }, || {
            album::ActiveModel {
                id: NotSet,
                title: Set(metadata.album.to_owned().unwrap()),
            }
        }, album::Column::Id).await?;
        if album_new_row.is_some() {
            trace!("{orig_filename}: new album generated: {album_new_row:?}");
        } else {
            trace!("{orig_filename}: album already exists: {album_id:?}");
        }

        // finally, insert track. compare based on audio_hash
        //
        // TODO: if track_new_row doesnt exist, throw an error, meaning that the track
        // already existed
        debug!("{orig_filename}: Filling track");
        let (track_new_row, track_id) = create_model_and_id(txn, Some(()), || {
            track::Column::AudioHash.eq(metadata.audiohash.unwrap().to_vec())
        }, || track::ActiveModel {
            id: Set(id),
            title: Set(metadata.title.to_owned().unwrap()),
            sort_name: Set(None),
            tags: {
                Set(if metadata.overflow.is_some() {
                    metadata.overflow.to_owned().unwrap().into()
                } else {
                    "{}".to_owned().into()
                })
            },
            audio_hash: Set(metadata.audiohash.unwrap().to_vec()),
            orig_fname: Set(orig_filename.clone()),
            album: Set(album_id),
            artist: Set(artist_id),
            cover_art: Set(cover_art_id),
            owner: Set(userid),
            disk: Set(metadata.disk_track.0),
            track: Set(metadata.disk_track.1),
        }, track::Column::Id).await?;
        if track_new_row.is_some() {
            trace!("{orig_filename}: track generated: {track_new_row:?}");
        } else {
            trace!("{orig_filename}: conflicting uuid found: {track_id:?}");
        }
        Ok(())
    })).await
}

// This abomination of traits, generics, and params is a generic function to
// possibly return a new model, and possibly return the id that the track will
// link to.
async fn create_model_and_id<
    DBModel,
    IsSome,
    ActiveModelType,
    IdColumn,
    ActiveModelFunc,
    FilterFunc,
    FilterFuncRet,
>(
    txn: &DatabaseTransaction,
    mdatainp: impl Borrow<Option<IsSome>>,
    filter: FilterFunc,
    active_model: ActiveModelFunc,
    id_column: IdColumn,
) -> Result<(Option<<DBModel as EntityTrait>::Model>, Option<Uuid>), DbErr>
where
    DBModel: EntityTrait<Column = IdColumn>,
    <<DBModel as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<Uuid>,
    ActiveModelFunc: FnOnce() -> ActiveModelType,
    ActiveModelType: ActiveModelTrait<Entity = DBModel> + ActiveModelBehavior + Send,
    FilterFunc: FnOnce() -> FilterFuncRet,
    FilterFuncRet: IntoCondition,
    <DBModel as EntityTrait>::Model: IntoActiveModel<ActiveModelType>,
    IdColumn: Copy {
    if mdatainp.borrow().is_some() {
        trace!("CMAI borrowed data is real");
        let id = DBModel::find().filter(filter()).one(txn).await?;
        if id.is_none() {
            trace!("CMAI no item found, bound by filter");
            let mut ret = active_model();
            let gen_id = if ret.is_not_set(id_column) {
                let gen_id = loop {
                    let uuid = Uuid::new_v4();
                    if DBModel::find_by_id(uuid).one(txn).await?.is_none() {
                        break uuid;
                    }
                };
                trace!("CMAI uuid generated: {gen_id}");
                ret.set(id_column, gen_id.into());
                gen_id
            } else {
                trace!("CMAI uuid already set");
                ret.get(id_column).unwrap().unwrap()
            };
            Ok((Some(ret.insert(txn).await?), Some(gen_id)))
        } else {
            trace!("CMAI item found, bound by filter");
            Ok((None, Some(id.unwrap().get(id_column).unwrap())))
        }
    } else {
        trace!("CMAI no borrowed data");
        Ok((None, None))
    }
}
