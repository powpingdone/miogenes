use glib::SendValue;
use gstreamer::{glib, ElementFactory};
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use image::GenericImageView;
use log::*;
use sea_orm::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};

// metadata parsed from the individual file
#[derive(Default)]
struct Metadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    overflow: Option<String>,
    blurhash: Option<Vec<u8>>,
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
                            queue = queue
                                .into_iter()
                                .filter(|task| !task.is_finished())
                                .collect();
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
                        move || get_metadata(orig_filename.as_str())
                    })
                    .await
                    .expect("join failure");
                    drop(permit);

                    if let Err(err) = mdata {
                        warn!("ERROR processing {orig_filename}: {err}");
                        // TODO: handle error
                        return;
                    }

                    insert_into_db(db, id, userid, mdata.unwrap()).await;
                }
            }))
            .unwrap();
    }

    gc.await.unwrap();
}

fn get_metadata(fname: impl AsRef<Path> + Clone) -> Result<Metadata, anyhow::Error> {
    // TODO: make this timeout configurable
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(10))?;
    let orig_path = fname.clone();
    let fname = glib::filename_to_uri(fname, None)?;
    debug!("{fname}: begin discovery");
    let data = discover.discover_uri(&fname)?;
    // TODO: implement errors for the rest (possibly, may not be needed)
    match data.result() {
        DiscovererResult::Ok => (),
        DiscovererResult::MissingPlugins => {
            anyhow::bail!("Missing plugin needed for file {fname}");
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
    debug!("{fname}: collecting tags");
    let mut mdata: Metadata = Default::default();
    // iterate through all tags
    let mut set = HashMap::new();
    for (tag, data) in data
        .container_streams()
        .iter()
        .map(|streaminfo| {
            let tags = streaminfo.tags();
            let mut ret = vec![];
            if let Some(tags) = tags {
                for (tag, value) in tags.iter() {
                    trace!("{fname}: tag proc'd \"{tag}\"");
                    ret.push((tag.to_owned(), value));
                }
            } else {
                debug!("{fname}: streaminfo.tags() produced a none");
            };
            ret
        })
        .flatten()
    {
        trace!("{fname}: tag \"{tag}\"");
        match tag.as_str() {
            // TODO: verify this is the right thing
            "image" => {
                let samp = data.get::<gstreamer::Sample>()?;
                if let Some(bufs) = samp.buffer_list() {
                    let img: Vec<u8> = {
                        let mut imgbuf: Vec<u8> = vec![];
                        for buf in bufs {
                            imgbuf = buf.iter_memories().fold(imgbuf, |mut accum, mem| {
                                // TODO: make this error better
                                let memread =
                                    mem.map_readable().expect("memory should be readable");
                                memread.iter().for_each(|x| accum.push(*x));
                                accum
                            })
                        }
                        imgbuf
                    };
                    mdata.imghash = Some(hash(&img));
                    trace!("{fname}: imghash: {:?}", mdata.imghash);

                    const BHASH_W: u32 = 128;
                    const BHASH_H: u32 = 128;
                    let dropped = image::load_from_memory(&img)?;
                    let shrunk_img = dropped.resize_exact(
                        BHASH_W,
                        BHASH_H,
                        image::imageops::FilterType::Nearest,
                    );
                    mdata.blurhash = Some(
                        blurhash::encode(6, 6, BHASH_W, BHASH_H, &shrunk_img.to_rgba8().into_vec())
                            .as_bytes()
                            .to_vec(),
                    );
                    trace!("{fname}: blurhash: {:?}", mdata.blurhash);
                } else {
                    anyhow::bail!("no buffer list found for image");
                }
            }

            "title" => {
                mdata.title = proc_tag(data);
                trace!("{fname}: title is |\"{:?}\"|", mdata.title)
            }
            "artist" => {
                mdata.artist = proc_tag(data);
                trace!("{fname}: artist is |\"{:?}\"|", mdata.artist)
            }
            "album" => {
                mdata.album = proc_tag(data);
                trace!("{fname}: album is |\"{:?}\"|", mdata.album)
            }

            _ => {
                let data = proc_tag(data);
                let ret = set.insert(tag.clone(), data.clone());
                trace!("{fname}: KV inserted ({tag}, {data:?}), replaced ({tag}, {ret:?})")
            }
        }
    }
    mdata.overflow = Some(serde_json::to_string(&set)?);
    if mdata.title.is_none() {
        // we sanitized the filename earlier, so we can unwrap here without a panic
        warn!("{fname}: this song has no \"title\" tag, using filename");
        mdata.title = Some(
            orig_path
                .as_ref()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned(),
        )
    }
    drop(discover);

    // audiohash
    trace!("{fname}: beginning audiohash");
    let elems = (
        ElementFactory::make("uridecodebin", Some("input"))?,
        ElementFactory::make("appsink", Some("sink"))?,
    );
    elems.0.set_property_from_str("uri", &fname);
    let pipeline = gstreamer::Pipeline::new(Some("extractor"));
    pipeline.add_many(&[&elems.0, &elems.1])?;
    gstreamer::Element::link_many(&[&elems.0, &elems.1])?;

    // sink extractor
    let m_loop = glib::MainLoop::new(None, false);
    let (tx, rx) = std::sync::mpsc::channel();
    let sink = elems
        .1
        .dynamic_cast::<AppSink>()
        .expect("failed dynamic cast to AppSink");
    sink.set_callbacks({
        let fname = fname.clone();
        let m_loop = m_loop.clone();
        let m_loop_fname = fname.clone();
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                trace!("{fname}: sink match from main loop entered");
                match sink.pull_sample() {
                    Ok(sample) => match sample.buffer_list() {
                        Some(buflist) => {
                            trace!("{fname}: found some buffers, sending over");
                            for buffer in buflist.iter() {
                                for mem in buffer.iter_memories() {
                                    mem.map_readable()
                                        // TODO: make this error better
                                        .expect("memory should be readable")
                                        .iter()
                                        .for_each(|ret| tx.send(*ret).unwrap());
                                }
                            }
                            Ok(gstreamer::FlowSuccess::Ok)
                        }
                        None => {
                            debug!("{fname}: failed to get buffer list as none was produced");
                            Err(gstreamer::FlowError::Error)
                        }
                    },
                    Err(err) => {
                        debug!("{fname}: failed to grab sample: {err}");
                        Err(gstreamer::FlowError::Error)
                    }
                }
            })
            .eos(move |_| {
                trace!("{m_loop_fname}: called exit");
                m_loop.quit();
            })
            .build()
    });

    pipeline.set_state(gstreamer::State::Playing)?;
    debug!("{fname}: entering waveform capture loop");
    m_loop.run();
    debug!("{fname}: exiting waveform capture loop");
    pipeline.set_state(gstreamer::State::Null)?;
    let waveform = rx.into_iter().collect::<Vec<_>>();
    mdata.audiohash = Some(hash(&waveform));
    trace!("{fname}: audiohash is {:?}", mdata.audiohash);

    Ok(mdata)
}

// TODO: maybe just directly copy the array
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

async fn insert_into_db(db: Arc<DatabaseConnection>, id: Uuid, userid: Uuid, metadata: Metadata) {}
