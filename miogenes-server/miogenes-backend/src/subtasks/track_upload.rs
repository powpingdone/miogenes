use gstreamer::glib;
use gstreamer::tags::GenericTagIter;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use image::GenericImageView;
use sea_orm::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::Path;
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
                tokio::select! {
                    task = rx_gc.recv() => {
                        if let None = task {
                            break;
                        }
                        queue.push(task.unwrap());
                    }
                    _ = sleep(Duration::from_secs(1)) => {
                        queue = queue.into_iter()
                                     .filter(|task| task.is_finished())
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

    let mut mdata: Metadata = Default::default();

    if let Some(tags) = data.tags() {
        let mut set = HashMap::new();
        for (tag, data) in tags.iter_generic() {
            match tag {
                "image" => {
                    let img = data.map(|x| x.get::<u8>().unwrap()).collect::<Vec<_>>();
                    let sha = Sha256::digest(&img);
                    // TODO: why
                    let mut actual_hash: [u8; 32] = Default::default();
                    for (hasharr, digested) in actual_hash.iter_mut().zip(sha.iter()) {
                        *hasharr = *digested;
                    }
                    mdata.imghash = Some(actual_hash);

                    let dropped = image::load_from_memory(&img)?;
                    let (w, h) = dropped.dimensions();
                    mdata.blurhash = Some(
                        blurhash::encode(6, 6, w, h, &dropped.to_rgba8().into_vec())
                            .as_bytes()
                            .to_vec(),
                    )
                }

                "title" => mdata.title = Some(proc_gen_tag_iter(data)),
                "artist" => mdata.artist = Some(proc_gen_tag_iter(data)),
                "album" => mdata.album = Some(proc_gen_tag_iter(data)),
                _ => {
                    set.insert(tag, proc_gen_tag_iter(data));
                }
            }
        }
        mdata.overflow = Some(serde_json::to_string(&set)?)
    }
    if let None = mdata.title {
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

    Ok(mdata)
}

fn proc_gen_tag_iter(data: GenericTagIter) -> String {
    data.fold("".to_owned(), |accum, x| {
        if let Ok(x) = x.get::<&str>() {
            accum + x
        } else if let Ok(x) = x.serialize() {
            accum + x.as_str()
        } else {
            accum
        }
    })
}

async fn insert_into_db(db: Arc<DatabaseConnection>, id: Uuid, userid: Uuid, metadata: Metadata) {}
