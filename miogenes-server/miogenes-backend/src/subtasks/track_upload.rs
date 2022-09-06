use gstreamer::glib;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
use sea_orm::prelude::*;
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

fn get_metadata(fname: impl AsRef<Path>) -> Result<Metadata, anyhow::Error> {
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(5))?;
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

    let mdata: Metadata = Default::default();

    if let Some(tags) = data.tags() {
        todo!()
    }

    Ok(mdata)
}

async fn insert_into_db(db: Arc<DatabaseConnection>, id: Uuid, userid: Uuid, metadata: Metadata) {}
