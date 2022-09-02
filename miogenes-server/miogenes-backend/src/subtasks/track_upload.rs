use gstreamer::glib;
use gstreamer_pbutils::prelude::*;
use sea_orm::prelude::*;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::sync::oneshot::channel as oneshot_channel;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

// metadata parsed from the individual file
struct TagMetadata {
    artist: String,
    title: String,
    album: String,
    overflow: String,
}

struct CacheMetadata {
    blurhash: Vec<u8>,
    imghash: [u8; 32],
    audiohash: [u8; 32],
}

struct Metadata {
    tags: oneshot::Receiver<Result<TagMetadata, anyhow::Error>>,
    cache: oneshot::Receiver<Result<CacheMetadata, anyhow::Error>>,
}

// TODO: upload process time limits
// TODO: tasks that are setup shouldn't be as blocking
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
                    let (mdata_tx, mdata_rx) = oneshot_channel();
                    let mdata = tokio::task::spawn_blocking({
                        let orig_filename = orig_filename.clone();
                        move || get_metadata(orig_filename.as_str(), mdata_tx)
                    });
                    let (cdata_tx, cdata_rx) = oneshot_channel();
                    let cache = tokio::task::spawn_blocking({
                        move || generate_cache(orig_filename.as_str(), cdata_tx)
                    });

                    let recvs = Metadata {
                        tags: mdata_rx,
                        cache: cdata_rx,
                    };
                    insert_into_db(db, id, userid, recvs).await;
                    mdata.await.unwrap();
                    cache.await.unwrap();
                }
            }))
            .unwrap();
    }

    gc.await.unwrap();
}

fn get_metadata(fname: impl AsRef<Path>, metadata_shot: oneshot::Sender<Result<TagMetadata, anyhow::Error>>) {
    metadata_shot.send(get_metadata_inner(fname)).ok().expect("broken oneshot tag sender");
}

fn get_metadata_inner(fname: impl AsRef<Path>) -> Result<TagMetadata, anyhow::Error> {
    let discover = gstreamer_pbutils::Discoverer::new(gstreamer::ClockTime::from_seconds(5))?;
    let fname = glib::filename_to_uri(fname, None)?;
    let data = discover.discover_uri(&fname)?;
    todo!()
}

fn generate_cache(
    fname: &str,
    metadata_shot: oneshot::Sender<Result<CacheMetadata, anyhow::Error>>,
) {
}

async fn insert_into_db(
    db: Arc<DatabaseConnection>,
    id: Uuid,
    userid: Uuid,
    metadata_shot: Metadata,
) {
    // wait for metadata
    let cache = metadata_shot.cache.await.unwrap();
    let tags = metadata_shot.tags.await.unwrap();
}
