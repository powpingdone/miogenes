use glib::SendValue;
use gstreamer::{glib, ElementFactory};
use gstreamer_app::AppSink;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use image::GenericImageView;
use sea_orm::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::channel;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

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
    db: Arc<DatabaseConnection>,
    mut rx: UnboundedReceiver<(Uuid, Uuid, String)>,
) {
    let (tx_gc, mut rx_gc) = unbounded_channel();

    let gc = tokio::spawn({
        async move {
            let mut queue: Vec<JoinHandle<_>> = vec![];
            loop {
                // TODO: use tokio::time::timeout
                tokio::select! {
                    task = rx_gc.recv() => {
                        if task.is_none() {
                            break;
                        }
                        queue.push(task.unwrap());
                    }
                    _ = sleep(Duration::from_secs(1)) => {
                        queue = queue.into_iter()
                                     .filter(|task| !task.is_finished())
                                     .collect();
                    }
                    else => { panic!("select failed!") },
                }
            }
        }
    });

    while let Some((id, userid, orig_filename)) = rx.recv().await {
        tx_gc
            .send(tokio::spawn({
                let db = db.clone();
                async move {
                    let mdata = tokio::task::spawn_blocking({
                        let fname = orig_filename.clone();
                        move || get_metadata(fname.as_str())
                    })
                    .await
                    .expect("join failure");

                    if let Err(_err) = mdata {
                        // TODO: log error
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
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(10))?;
    let orig_path = fname.clone();
    let fname = glib::filename_to_uri(fname, None)?;
    let data = discover.discover_uri(&fname)?;

    // TODO: implement errors for the rest (possibly, may not be needed)
    match data.result() {
        DiscovererResult::Ok => (),
        DiscovererResult::MissingPlugins => {
            anyhow::bail!("Missing plugin needed for file {fname}");
        }

        DiscovererResult::UriInvalid => unimplemented!(),
        DiscovererResult::Error => unimplemented!(),
        DiscovererResult::Timeout => unimplemented!(),
        DiscovererResult::Busy => unimplemented!(),
        _other => panic!("unhandled enum: {_other:?}"),
    }

    // tag metadata
    let mut mdata: Metadata = Default::default();
    let tags = data
        .container_streams()
        .iter()
        .fold(vec![], |mut accum, streaminfo| {
            let tags = streaminfo.tags();
            if let Some(tags) = tags {
                for (tag, value) in tags.iter() {
                    accum.push((tag.to_owned(), value))
                }
            };
            accum
        });
    let mut set = HashMap::new();
    for (tag, data) in tags {
        match tag.as_str() {
            "image" => {
                if let Ok(samp) = data.get::<gstreamer::Sample>() {
                    if let Some(buf) = samp.buffer() {
                        let img: Vec<u8> = buf.iter_memories().fold(vec![], |mut accum, mem| {
                            let memread = mem.map_readable().expect("memory should be readable");
                            memread.iter().for_each(|x| accum.push(*x));
                            accum
                        });
                        mdata.imghash = Some(hash(&img));

                        let dropped = image::load_from_memory(&img)?;
                        let (w, h) = dropped.dimensions();
                        mdata.blurhash = Some(
                            blurhash::encode(6, 6, w, h, &dropped.to_rgba8().into_vec())
                                .as_bytes()
                                .to_vec(),
                        )
                    }
                }
            }

            "title" => mdata.title = proc_tag(data),
            "artist" => mdata.artist = proc_tag(data),
            "album" => mdata.album = proc_tag(data),
            _ => {
                set.insert(tag, proc_tag(data));
            }
        }
    }
    mdata.overflow = Some(serde_json::to_string(&set)?);
    if mdata.title.is_none() {
        // we sanitized the filename earlier, so we can unwrap here without a panic
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
    let elems = (
        ElementFactory::make("uridecodebin", Some("input"))?,
        ElementFactory::make("appsink", Some("sink"))?,
    );
    elems.0.set_property_from_str("uri", &fname);
    let pipeline = gstreamer::Pipeline::new(Some("extractor"));
    pipeline.add_many(&[&elems.0, &elems.1])?;
    gstreamer::Element::link_many(&[&elems.0, &elems.1])?;

    // sink extractor
    let (tx, rx) = channel();
    let sink = elems.1.dynamic_cast::<AppSink>();
    if sink.is_err() {
        anyhow::bail!("failed to dynamic_cast to AppSink");
    }
    let sink = sink.unwrap();
    sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| match sink.pull_sample() {
                Ok(sample) => match sample.buffer_list() {
                    Some(buf) => {
                        for buffer in buf.iter() {
                            for mem in buffer.iter_memories() {
                                mem.map_readable()
                                    .expect("memory should be readable")
                                    .iter()
                                    .for_each(|ret| tx.send(*ret).unwrap());
                            }
                        }
                        Ok(gstreamer::FlowSuccess::Ok)
                    }
                    None => Err(gstreamer::FlowError::Error),
                },
                Err(_) => Err(gstreamer::FlowError::NotNegotiated),
            })
            .build(),
    );

    let m_loop = glib::MainLoop::new(None, false);
    pipeline.set_state(gstreamer::State::Playing)?;
    m_loop.run();
    pipeline.set_state(gstreamer::State::Null)?;
    let waveform = rx.into_iter().collect::<Vec<_>>();
    mdata.audiohash = Some(hash(&waveform));

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
        Some(x)
    } else if let Ok(x) = data.serialize() {
        Some(x.to_string())
    } else {
        None
    }
}

async fn insert_into_db(db: Arc<DatabaseConnection>, id: Uuid, userid: Uuid, metadata: Metadata) {}
