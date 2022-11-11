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
        .route("/tu", put(track_upload))
        .route("/td", put(track_delete))
}

async fn track_upload(
    Extension(state): Extension<Arc<crate::MioState>>,
    mut payload: Multipart,
    Extension(userid): Extension<Uuid>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let mut ret_ids: Vec<(Uuid, Uuid, String)> = vec![];

    // collect file
    loop {
        // get field
        trace!("GET /track/tu getting field");
        let field = payload.next_field().await;
        if field.is_err() {
            info!("GET /track/tu could not fetch field during request");
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
        debug!("GET /track/tu generating UUID");
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
                    trace!("GET /track/tu opened file {fname}");
                    file = x;
                    break;
                }
                Err(err) => {
                    if err.kind() == ErrorKind::AlreadyExists {
                        trace!("GET /track/tu file already exists");
                        continue;
                    }
                    error!("GET /track/tu failed to open file: {err}");
                    rm_files(ret_ids.iter().map(|x| x.0).collect()).await;
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(crate::MioError::i_s_e()),
                    ));
                }
            }
        }

        // get original filename
        let orig_filename = sanitize_filename::sanitize(field.file_name().map_or_else(
            || {
                trace!("GET /track/tu generated fname with uuid");
                uuid.to_string()
            },
            |ret| {
                trace!("GET /track/tu used orig filename: {ret}");
                ret.to_owned()
            },
        ));

        info!("GET /track/tu filename and uuid used: \"{fname}\" {uuid}");

        // download the file
        // TODO: filesize limits
        // TODO: maybe don't panic on filesystem errors(?)
        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    debug!("GET /track/tu {uuid}: writing {} bytes", chunk.len());
                    file.write_all(&chunk)
                        .await
                        .expect("Failed to write to file: {}");
                }
                // No more data
                Ok(None) => break,
                // TODO: log this error
                Err(err) => {
                    // delete failed upload, as well as all other uploads per this req
                    info!("GET /track/tu failed upload for {uuid}: {err}");
                    trace!("GET /track/tu flushing {uuid}");
                    file.flush()
                        .await
                        .expect("Failed to flush uploaded file: {}");
                    drop(file);
                    // push blank id, just to delete uuid
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

        ret_ids.push((uuid, userid, orig_filename));
    }

    #[derive(Serialize)]
    struct UploadReturn {
        uuid: Vec<Uuid>,
    }

    Ok((
        StatusCode::PROCESSING,
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
        trace!("RM_FILES deleting {uuid}");
        remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid))
            .await
            .expect("unable to remove file {}");
    }
}

#[derive(Debug, Deserialize)]
struct DeleteQuery {
    pub id: Uuid,
}

async fn track_delete(
    Extension(state): Extension<Arc<crate::MioState>>,
    Query(id): Query<DeleteQuery>,
    Extension(userid): Extension<Uuid>,
) -> impl IntoResponse {
    todo!()
}
