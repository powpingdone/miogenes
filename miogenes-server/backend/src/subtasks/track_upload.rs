use glib::SendValue;
use gstreamer::glib;
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use log::*;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use uuid::*;

use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use crate::DATA_DIR;

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
}

// TODO: upload process time limits
// TODO: size limits
pub async fn track_upload_server(
    state: Arc<crate::MioState>,
    mut rx: UnboundedReceiver<(Uuid, Uuid, String)>,
) {
    debug!("starting track_upload_server");
    let (tx_gc, mut rx_gc) = unbounded_channel();

    // joiner, this cleans up the join handles along with any
    // errors that might have been propagated
    let gc = tokio::spawn({
        async move {
            let mut queue: Vec<JoinHandle<_>> = vec![];
            loop {
                match timeout(Duration::from_secs(10), rx_gc.recv()).await {
                    Ok(recv) => {
                        if recv.is_none() {
                            // if it returns none, this means the channel closed
                            debug!("closing channel");
                            break;
                        }
                        trace!("recv'd task");
                        queue.push(recv.unwrap());
                    }
                    Err(_) => {
                        if !queue.is_empty() {
                            trace!("GC activate");
                            let prev_len = queue.len();
                            queue.retain(|task| !task.is_finished());
                            trace!("cleaned {}", prev_len - queue.len());
                        }
                    }
                }
            }
        }
    });

    // recviver that schedules the processing tasks
    while let Some((id, userid, orig_filename)) = rx.recv().await {
        trace!("sending task {id}");
        tx_gc
            .send(tokio::spawn({
                let db = state.db.clone();
                let permit = state.lim.clone();
                async move {
                    let permit = permit.acquire().await.unwrap();
                    // TODO: possibly log "Starting task..."
                    let mdata = tokio::task::spawn_blocking({
                        let orig_filename = orig_filename.clone();
                        move || {
                            get_metadata(
                                format!("{}{}", DATA_DIR.get().unwrap(), id).as_str(),
                                orig_filename.as_str(),
                            )
                        }
                    })
                    .await
                    .expect("join failure");
                    drop(permit);

                    if let Err(err) = mdata {
                        warn!("ERROR processing {orig_filename}: {err}");
                        // TODO: handle error
                        return;
                    }

                    if let Err(err) = insert_into_db(db, id, userid, mdata.unwrap()).await {
                        error!("ERROR querying the database for {orig_filename}: {err}");
                        panic!("querying the database failed");
                    }
                }
            }))
            .unwrap();
    }

    gc.await.unwrap();
}

fn get_metadata(fname: &str, orig_path: &str) -> Result<Metadata, anyhow::Error> {
    // TODO: make this timeout configurable
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(10))?;
    trace!("{orig_path}: creating discoverer");
    let fname = glib::filename_to_uri(Path::new(fname).absolutize()?, None)?;
    trace!("{orig_path}: new uri created: '{fname}'");
    debug!("{orig_path}: begin discovery");
    let data = discover.discover_uri(&fname)?;
    // TODO: implement errors for the rest (possibly, may not be needed)
    trace!("{orig_path}: result: {:?}", data.result());
    match data.result() {
        DiscovererResult::Ok => (),
        DiscovererResult::MissingPlugins => {
            anyhow::bail!("Missing plugin needed for file {orig_path}");
        }
        DiscovererResult::Timeout => {
            anyhow::bail!("Timeout reached for processing tags")
        }

        // these branches *shouldn't* fail.
        // Busy -> each discoverer is in it's own thread, where it only reads one file
        // UriInvalid -> fname is produced via glib::filename_to_uri
        // Error -> discover_uri can return an error, so this shouldn't happen here
        DiscovererResult::Busy => unimplemented!(),
        DiscovererResult::UriInvalid => unimplemented!(),
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
                            buf.map_readable()
                                .expect("memory should be readable: {}")
                                .iter()
                                .for_each(|x| imgbuf.push(*x));
                        }
                        imgbuf
                    };
                    mdata.imghash = Some(hash(&img));
                    trace!("{orig_path}: imghash: {:?}", mdata.imghash);

                    // convert to webp
                    let dropped = image::load_from_memory(&img)?;
                    let conv = dropped
                        .as_rgb8()
                        .ok_or(anyhow::anyhow!("failed to convert to rgb8"))?;
                    let mut hold = Cursor::new(vec![]);
                    conv.write_to(&mut hold, image::ImageFormat::WebP)?;
                    mdata.img = Some(hold.into_inner());
                } else {
                    anyhow::bail!("no buffer found for image");
                }
            }

            "title" => {
                mdata.title = proc_tag(data);
                trace!("{orig_path}: title is {:?}", mdata.title)
            }
            "artist" => {
                mdata.artist = proc_tag(data);
                trace!("{orig_path}: artist is {:?}", mdata.artist)
            }
            "album" => {
                mdata.album = proc_tag(data);
                trace!("{orig_path}: album is {:?}", mdata.album)
            }

            _ => {
                let data = proc_tag(data);
                let ret = set.insert(tag.clone(), data.clone());
                trace!("{orig_path}: KV inserted ({tag}, {data:?}), replaced ({tag}, {ret:?})")
            }
        }
    }
    mdata.overflow = Some(serde_json::to_string(&set)?);
    if mdata.title.is_none() {
        warn!("{orig_path}: this song has no \"title\" tag, using filename");
        mdata.title = Some(orig_path.to_owned())
    }
    drop(discover);

    // audiohash
    trace!("{orig_path}: beginning audiohash");
    let pipeline = gstreamer::parse_launch(&format!(
        "uridecodebin3 uri={fname} ! audioconvert ! appsink name=sink",
    ))?
    .downcast::<gstreamer::Pipeline>()
    .expect("Expected a gst::Pipeline");

    // sink extractor
    // TODO: either do a shared memory thing or Oneshot it
    let (tx, rx) = std::sync::mpsc::channel();
    let sink = pipeline
        .by_name("sink")
        .expect("sink element not found")
        .dynamic_cast::<AppSink>()
        .expect("failed dynamic cast to AppSink");
    sink.set_property("sync", false);
    sink.set_callbacks({
        let orig_path = orig_path.to_owned();
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                trace!("{orig_path}: sink match from main loop entered");
                match sink.pull_sample() {
                    Ok(sample) => match sample.buffer() {
                        Some(buflist) => {
                            trace!("{orig_path}: found some buffers, sending over");
                            tx.send(
                                buflist
                                    .iter_memories()
                                    .flat_map(|buf| {
                                        buf.map_readable()
                                            .expect("memory should be readable")
                                            .iter()
                                            .copied()
                                            .collect::<Vec<_>>()
                                    })
                                    .collect::<Vec<_>>(),
                            )
                            .unwrap();
                            Err(gstreamer::FlowError::Eos)
                        }
                        None => {
                            debug!("{orig_path}: failed to get buffer list as none was produced");
                            Err(gstreamer::FlowError::Error)
                        }
                    },
                    Err(err) => {
                        debug!("{orig_path}: failed to grab sample: {err}");
                        Err(gstreamer::FlowError::Error)
                    }
                }
            })
            .build()
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

// copy the hash of one value into a regular array
// since sha2 uses GenericArray's, this is done via just
// manually iterating through it
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
    db: sea_orm::DatabaseConnection,
    id: Uuid,
    userid: Uuid,
    metadata: Metadata,
) -> Result<(), anyhow::Error> {
    todo!();
}
