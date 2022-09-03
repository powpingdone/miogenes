use sea_orm::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::sync::oneshot;
use tokio::sync::oneshot::channel as oneshot_channel;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration};

// metadata parsed from the individual file
#[derive(Debug)]
struct TagMetadata {
    artist: Option<String>,
    title: Option<String>,
    album: Option<String>,
    overflow: Option<String>,
    blurhash: Option<Vec<u8>>,
    imghash: Option<[u8; 32]>,
    audiohash: Option<[u8; 32]>,
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
                    let mdata = tokio::task::spawn_blocking({
                        move || get_metadata(orig_filename.as_str())
                    });

                    let tags = mdata.await.unwrap();
                    if let Ok(tags) = tags {
                        insert_into_db(db, id, userid, tags).await;
                    } else {
                        todo!();
                    }
                }
            }))
            .unwrap();
    }

    gc.await.unwrap();
}

fn get_metadata(fname: impl AsRef<Path>) -> Result<TagMetadata, anyhow::Error> {
    use ffmpeg::codec::Context;
    use ffmpeg::media::Type;
    use ffmpeg::format::stream::disposition::Disposition;

    let ffile = ffmpeg::format::input(&fname)?;
    let mut tags = TagMetadata {
        artist: None,
        title: None,
        album: None,
        overflow: None,
        blurhash: None,
        imghash: None,
        audiohash: None,
    };

    // metadata
    let mdata = ffile.metadata();
    let mut extra_tags = HashMap::new();
    for (tag, data) in mdata.iter() {
        match tag {
            "title" => tags.title = Some(data.to_owned()),
            "artist" => tags.artist = Some(data.to_owned()),
            "album" => tags.album = Some(data.to_owned()),
            _ => {
                // TODO: insert does not allow for duplicate keys
                // should possibly handle that...?
                // because we're not handling it, change return type to ()
                let _ = extra_tags.insert(tag, data);
            }
        }
    }
    tags.overflow = Some(serde_json::to_string(&extra_tags)?);
    drop(extra_tags);

    // hashes
    let streams = ffile.streams();
    for i in streams {
        let codec_parsed = Context::from_parameters(i.parameters())?;
        match codec_parsed.medium() {
            Type::Video /* cover art */ => {
                if i.disposition() & Disposition::ATTACHED_PIC == Disposition::ATTACHED_PIC {
                    // writeout the img and do computation
                    todo!();
                }
                
            }
            Type::Audio => {
                // checksum the img
            },
            _ => {}, // do nothing
        }
    }

    todo!()
}

async fn insert_into_db(db: Arc<DatabaseConnection>, id: Uuid, userid: Uuid, tags: TagMetadata) {
    todo!()
}
