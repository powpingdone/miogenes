use crate::db::uuid_serialize;
use crate::db::write_transaction;
use crate::*;
use anyhow::anyhow;
use axum::http::StatusCode;
use glib::SendValue;
use gstreamer::glib;
use gstreamer_pbutils::prelude::*;
use gstreamer_pbutils::DiscovererResult;
#[allow(unused)]
use log::*;
use path_absolutize::Absolutize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Debug;
use std::io::Cursor;
use std::path::Path;
use std::path::PathBuf;
use uuid::*;

// metadata parsed from the individual file
#[derive(Default)]
struct Metadata {
    title: String,
    other_tags: String,
    artist: Option<String>,
    artist_sort: Option<String>,
    album: Option<String>,
    album_sort: Option<String>,
    img: Option<(Vec<u8>, [u8; 32])>,
    disk_track: (Option<i32>, Option<i32>),
}

// TODO: upload process time limits
//
// TODO: size limits
pub async fn track_upload_process(
    state: MioState,
    id: Uuid,
    path: PathBuf,
    dir: String,
    userid: Uuid,
    orig_filename: String,
) -> Result<(), MioInnerError> {
    // process metadata
    let mdata = tokio::task::spawn_blocking({
        let orig_filename = orig_filename.clone();
        move || get_metadata(path, orig_filename)
    })
    .await
    .map_err(|err| {
        MioInnerError::TrackProcessingError(
            anyhow!("failed to run task: {err}"),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    })?
    .map_err(|err| MioInnerError::TrackProcessingError(err, StatusCode::BAD_REQUEST))?;

    // Quite Ok Audio encoding insert into the database
    insert_into_db(state.db, id, userid, dir, mdata, orig_filename).await
}

fn get_metadata(fname: PathBuf, orig_path: String) -> Result<Metadata, anyhow::Error> {
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
        }
        DiscovererResult::Timeout => {
            anyhow::bail!("Timeout reached for processing tags");
        }
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

    // tag metadata, iterate through all tags
    debug!("{orig_path}: collecting tags");
    let mut title = None;
    let mut artist = None;
    let mut artist_sort = None;
    let mut album = None;
    let mut album_sort = None;
    let mut img = None;
    let mut disk_track = (None, None);
    let mut set = HashMap::new();
    for (tag, data) in data.audio_streams().iter().flat_map(|streaminfo| {
        let tags = streaminfo.tags();
        let mut ret = vec![];
        match tags {
            Some(tags) => {
                for (tag, value) in tags.iter() {
                    trace!("{orig_path}: tag proc'd \"{tag}\"");
                    ret.push((tag.to_owned(), value));
                }
            }
            None => debug!("{orig_path}: streaminfo.tags() produced a none"),
        };
        ret
    }) {
        trace!("{orig_path}: tag \"{tag}\"");
        match tag.as_str() {
            "image" => {
                let samp = data.get::<gstreamer::Sample>()?;
                if let Some(bufs) = samp.buffer() {
                    let img_raw = {
                        let mut imgbuf = vec![];
                        for buf in bufs {
                            buf.map_readable()
                                .expect("image memory should be readable: {}")
                                .iter()
                                .for_each(|x| imgbuf.push(*x));
                        }
                        imgbuf
                    };

                    // convert to webp
                    let dropped = image::load_from_memory(&img_raw)?;
                    let conv = dropped.into_rgb8();
                    let mut hold = Cursor::new(vec![]);
                    conv.write_to(&mut hold, image::ImageFormat::WebP)?;
                    let img_buf = hold.into_inner();
                    let imghash = hash(&img_buf);
                    trace!("{orig_path}: imghash: {:?}", imghash);
                    img = Some((img_buf, imghash));
                } else {
                    anyhow::bail!("no buffer found for image");
                }
            }
            "title" => {
                title = proc_tag(data);
                trace!("{orig_path}: title is {:?}", title)
            }
            "artist" => {
                artist = proc_tag(data);
                trace!("{orig_path}: artist is {:?}", artist)
            }
            "artist-sortname" => {
                artist_sort = proc_tag(data);
                trace!("{orig_path}: artist sortname is {:?}", artist_sort);
            }
            "album" => {
                album = proc_tag(data);
                trace!("{orig_path}: album is {:?}", album)
            }
            "album-sortname" => {
                album_sort = proc_tag(data);
                trace!("{orig_path}: album sortname is {:?}", album_sort);
            }
            "album-disc-number" | "track-number" => {
                let mut_info = if tag.as_str() == "album-disc-number" {
                    &mut disk_track.0
                } else {
                    &mut disk_track.1
                };
                *mut_info = proc_tag(data).and_then(|inp| match inp.parse() {
                    Ok(ok) => Some(ok),
                    Err(err) => {
                        debug!("{orig_path}: error parsing int out {err}");
                        None
                    }
                });
                trace!("{orig_path}: disk_track is {:?}", disk_track)
            }
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

                    let (data, ret) = ({ truncate(format!("{data:?}")) }, {
                        truncate(format!("{ret:?}"))
                    });
                    trace!("{orig_path}: KV inserted ({tag}, {data}), replaced ({tag}, {ret})");
                }
            }
        }
    }
    let other_tags = if !set.is_empty() {
        serde_json::to_string(
            &set.into_iter()
                .map(|(a, b)| (a.to_string(), b))
                .collect::<HashMap<_, _>>(),
        )?
    } else {
        "{}".to_string()
    };
    let title = title.unwrap_or_else(|| {
        warn!("{orig_path}: this song has no \"title\" tag, using filename");
        orig_path.to_string()
    });
    Ok(Metadata {
        title,
        other_tags,
        artist,
        artist_sort,
        album,
        album_sort,
        img,
        disk_track,
    })
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

fn create_qoa(file: PathBuf) -> Result<Uuid, anyhow::Error> {
    todo!()
}

async fn insert_into_db(
    db: SqlitePool,
    id: Uuid,
    userid: Uuid,
    dir: String,
    metadata: Metadata,
    orig_filename: String,
) -> Result<(), MioInnerError> {
    let mut conn = db.acquire().await?;
    write_transaction(&mut conn, |txn| {
        Box::pin(async move {
            // insert cover art, check against img_hash
            let cover_art_id = {
                if let Some(cover_hash) = metadata.img.as_ref().map(|x| x.1) {
                    let q = cover_hash.as_slice();
                    match sqlx::query!(
                        "SELECT id FROM cover_art
                        WHERE img_hash = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
                        Some(x) => Some(uuid_serialize(&x.id)?),
                        None => {
                            let id = Uuid::new_v4();
                            let (webm_blob, img_hash_hold) = metadata.img.unwrap();
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
                        }
                    }
                } else {
                    None
                }
            };

            // insert artist, check on artist name
            let artist_id = {
                if let Some(q) = metadata.artist.as_ref() {
                    match sqlx::query!(
                        "SELECT id FROM artist
                        WHERE artist_name = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
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
                        }
                    }
                } else {
                    None
                }
            };

            // insert album, check on album title
            let album_id = {
                if let Some(q) = metadata.album.as_ref() {
                    match sqlx::query!(
                        "SELECT id FROM album
                        WHERE title = ?;",
                        q
                    )
                    .fetch_optional(&mut *txn)
                    .await?
                    {
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
                        }
                    }
                } else {
                    None
                }
            };

            // insert track, check on audiohash
            let other_tags = metadata.other_tags;
            sqlx::query!(
                "INSERT INTO track 
                    (id,
                    title, 
                    disk, 
                    track, 
                    tags, 
                    orig_fname, 
                    album, 
                    artist, 
                    cover_art, 
                    owner,
                    path) 
                VALUES (?,?,?,?,?,?,?,?,?,?,?);",
                id,
                metadata.title,
                metadata.disk_track.0,
                metadata.disk_track.1,
                other_tags,
                orig_filename,
                album_id,
                artist_id,
                cover_art_id,
                userid,
                dir
            )
            .execute(&mut *txn)
            .await?;
            trace!("{orig_filename}: new track created: {id}");
            Ok(())
        })
    })
    .await
}
