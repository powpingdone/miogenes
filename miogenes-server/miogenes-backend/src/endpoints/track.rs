use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use log::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs::{remove_file, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use uuid::Uuid;

pub fn routes() -> Router {
    Router::new()
        .route("/ti", get(track_info))
        .route("/tu", put(track_upload))
}

#[derive(Debug, Deserialize)]
struct TInfoQuery {
    #[serde(rename = "tr")]
    trackid: Uuid,
}

async fn track_info(
    state: Extension<Arc<crate::MioState>>,
    key: Query<crate::User>,
    track: Query<TInfoQuery>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let userid = crate::login_check(state.db.clone(), key.0).await?;

    // contact the database and query it
    debug!("/track_info {}: querying db", track.trackid);
    let resp: Result<Option<Result<(), String>>, String> = Ok(Some(Ok(())));

    match resp {
        // database fails to talk
        Err(err) => {
            error!(
                "/track_info database query returned an error {}: {err}",
                track.trackid
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(crate::MioError {
                    msg: format!("database error for {}", track.trackid),
                }),
            ))
        }
        Ok(resp) => match resp {
            // track doesn't exist
            None => {
                debug!("/track_info {}: track not found", track.trackid);
                Err((
                    StatusCode::NOT_FOUND,
                    Json(crate::MioError {
                        msg: format!("no track found for {}", track.trackid),
                    }),
                ))
            }
            Some(content) => match serde_json::to_string(&content) {
                Ok(json) => {
                    trace!("/track_info {}: json = {json}", track.trackid);
                    Ok((StatusCode::OK, Json(json)))
                }
                // somehow, serialization failed
                Err(err) => {
                    error!("/track_info serialization failure {}: {err}", track.trackid);
                    Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(crate::MioError {
                            msg: format!("internal serialization error for {}", track.trackid),
                        }),
                    ))
                }
            },
        },
    }
}

async fn track_upload(
    state: Extension<Arc<crate::MioState>>,
    Query(key): Query<crate::User>,
    mut payload: Multipart,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let userid = crate::login_check(state.db.clone(), key).await?;
    let mut ret_ids: Vec<(Uuid, Uuid, String)> = vec![];

    // collect file
    loop {
        // get field
        trace!("/track_upload getting field");
        let field = payload.next_field().await;
        if field.is_err() {
            info!("/track_upload could not fetch field during request");
            rm_files(ret_ids.iter().map(|x| x.0).collect()).await;
            return Err((
                StatusCode::BAD_REQUEST,
                Json(crate::MioError {
                    msg: "invalid or corrupt request".to_owned(),
                }),
            ));
        }
        let field = field.unwrap();
        if field.is_none() {
            break;
        }
        let mut field = field.unwrap();

        // TODO: store the filename for dumping purposes
        // find a unique id for the track
        debug!("/track_upload generating UUID");
        let mut uuid;
        let mut file: File;
        let mut fname;
        loop {
            uuid = Uuid::new_v4();
            fname = format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid);
            // check if file is already taken
            let check = OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(fname.clone())
                .await;
            match check {
                Ok(x) => {
                    trace!("/track_upload opened file {fname}");
                    file = x;
                    break;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::AlreadyExists {
                        trace!("/track_upload file already exists");
                        continue;
                    }
                    rm_files(ret_ids.iter().map(|x| x.0).collect()).await;
                    panic!("Failed to open file for writing during an upload: {err}");
                }
            }
        }

        // get original filename
        let orig_filename = sanitize_filename::sanitize(field.file_name().map_or_else(
            || {
                trace!("generated fname with uuid");
                uuid.to_string()
            },
            |ret| {
                trace!("used orig filename: {ret}");
                ret.to_string()
            },
        ));

        info!("/track_upload filename and uuid used: \"{fname}\" {uuid}");

        // download the file
        // TODO: filesize limits
        // TODO: maybe don't panic on filesystem errors(?)
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    debug!("/track_upload {uuid}: writing {} bytes", chunk.len());
                    file.write_all(&chunk)
                        .await
                        .expect("Failed to write to file: {}");
                }
                // No more data
                Ok(None) => break,
                // TODO: log this error
                Err(err) => {
                    // delete failed upload
                    info!("/track_upload failed upload for {uuid}: {err}");
                    trace!("/track_upload flushing {uuid}");
                    file.flush()
                        .await
                        .expect("Failed to flush uploaded file: {}");
                    drop(file);
                    ret_ids.push((uuid, Uuid::nil(), "".to_owned()));
                    rm_files(ret_ids.iter().map(|x| x.0).collect()).await;

                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(crate::MioError {
                            msg: "invalid or corrupt request".to_owned(),
                        }),
                    ));
                }
            }
        }

        ret_ids.push((uuid, /* userid */ Uuid::nil(), orig_filename));
    }

    #[derive(Serialize)]
    struct UploadReturn {
        uuid: Vec<Uuid>,
    }

    Ok((
        StatusCode::OK,
        Json(UploadReturn {
            uuid: ret_ids
                .into_iter()
                .map(|x| {
                    let ret = x.0;
                    // set off tasks to process files
                    state.proc_tracks_tx.send(x).unwrap();
                    ret
                })
                .collect::<Vec<_>>(),
        }),
    ))
}

// rm's file when track_upload errors out
async fn rm_files(paths: Vec<Uuid>) {
    for uuid in paths {
        trace!("/track_upload deleting {uuid}");
        remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid))
            .await
            .expect("unable to remove file {}");
    }
}

