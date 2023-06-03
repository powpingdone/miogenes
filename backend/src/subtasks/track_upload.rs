use crate::db::uuid_serialize;
use crate::*;
use axum::http::StatusCode;
use glib::SendValue;
use gstreamer::glib;
use gstreamer::glib::user_config_dir;
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use log::*;
use path_absolutize::Absolutize;
use sha2::{
    Digest,
    Sha256,
};
use sqlx::Connection;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use uuid::*;

// metadata parsed from the individual file
#[derive(Default)]
struct Metadata {
    artist: Option<String>,
    artist_sort: Option<String>,
    title: Option<String>,
    album: Option<String>,
    album_sort: Option<String>,
    other_tags: Option<String>,
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
) -> Result<(), MioInnerError> {
    // process metadata
    let permit = state.lim.acquire().await.unwrap();
    let mdata = tokio::task::spawn_blocking({
        let orig_filename = orig_filename.clone();
        move || get_metadata(format!("{}{}", DATA_DIR.get().unwrap(), id), orig_filename)
    }).await.expect("join failure").map_err(|err| {
        MioInnerError::TrackProcessingError(err, StatusCode::BAD_REQUEST)
    })?;
    drop(permit);

    // insert into the database
    insert_into_db(state.db, id, userid, mdata, orig_filename).await
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
                *mut_info = proc_tag(data).and_then(|inp| match inp.parse() {
                    Ok(ok) => Some(ok),
                    Err(err) => {
                        debug!("{orig_path}: error parsing int out {err}");
                        None
                    },
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
        mdata.other_tags =
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
) -> Result<(), MioInnerError> {
    db.acquire().await?.transaction(|txn| {
        Box::pin(async move {
            // insert cover art, check against img_hash
            let cover_art_id = {
                if let Some(cover_hash) = metadata.imghash.as_ref() {
                    let q = cover_hash.as_slice();
                    match sqlx::query!("SELECT id FROM cover_art
                            WHERE img_hash = ?;", q)
                        .fetch_optional(&mut *txn)
                        .await? {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let webm_blob = metadata.img.unwrap();
                            let img_hash_hold = metadata.imghash.unwrap();
                            let img_hash = img_hash_hold.as_slice();
                            sqlx::query!(
                                "INSERT INTO cover_art
                                    (id, webm_blob, img_hash)
                                    VALUES (?, ?, ?);",
                                id,
                                webm_blob,
                                img_hash
                            )
                                .execute(&mut *txn)
                                .await?;
                            trace!("{orig_filename}: new artist generated: {id}");
                            Some(id)
                        },
                    }
                } else {
                    None
                }
            };

            // insert artist, check on artist name
            let artist_id = {
                if let Some(q) = metadata.artist.as_ref() {
                    match sqlx::query!("SELECT id FROM artist
                            WHERE artist_name = ?;", q)
                        .fetch_optional(&mut *txn)
                        .await? {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let artist_name = metadata.artist.unwrap();
                            sqlx::query!(
                                "INSERT INTO artist
                                    (id, artist_name, sort_name)
                                    VALUES (?, ?, ?);",
                                id,
                                artist_name,
                                metadata.artist_sort
                            )
                                .execute(&mut *txn)
                                .await?;
                            trace!("{orig_filename}: new artist generated: {id}");
                            Some(id)
                        },
                    }
                } else {
                    None
                }
            };

            // insert album, check on album title
            let album_id = {
                if let Some(q) = metadata.album.as_ref() {
                    match sqlx::query!("SELECT id FROM album
                            WHERE title = ?;", q)
                        .fetch_optional(&mut *txn)
                        .await? {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let title = metadata.album.unwrap();
                            sqlx::query!(
                                "INSERT INTO album
                                    (id, title, sort_title)
                                    VALUES (?, ?, ?);",
                                id,
                                title,
                                metadata.album_sort
                            )
                                .execute(&mut *txn)
                                .await?;
                            trace!("{orig_filename}: new album generated: {id}");
                            Some(id)
                        },
                    }
                } else {
                    None
                }
            };

            // insert track, check on audiohash
            let hold = metadata.audiohash.unwrap();
            let audiohash = hold.as_slice();
            match sqlx::query!(
                "SELECT id FROM track
                WHERE owner = ? AND audio_hash = ?;",
                userid,
                audiohash
            )
                .fetch_optional(&mut *txn)
                .await? {
                Some(x) => {
                    return Err(
                        MioInnerError::TrackProcessingError(
                            anyhow::anyhow!(
                                "this track already seems to be in conflict with {}, not uploading",
                                uuid_serialize(&x.id)?
                            ),
                            StatusCode::CONFLICT,
                        ),
                    )
                },
                None => {
                    let id = Uuid::new_v4();
                    let ahold = metadata.audiohash.unwrap();
                    let audiohash = ahold.as_slice(); 
                    sqlx::query!(
                        "INSERT INTO track 
                            (id, 
                            title, 
                            disk, 
                            track, 
                            tags, 
                            audio_hash, 
                            orig_fname, 
                            album, 
                            artist, 
                            cover_art, 
                            owner) 
                        VALUES (?,?,?,?,?,?,?,?,?,?,?);",
                        id,
                        metadata.title,
                        metadata.disk_track.0,
                        metadata.disk_track.1,
                        metadata.other_tags.unwrap(),
                        audiohash,
                        orig_filename,
                        album_id,
                        artist_id,
                        cover_art_id,
                        userid,
                    );
                    trace!("{orig_filename}: new track created: {id}")
                },
            }
            Ok(())
        })
    }).await
}
