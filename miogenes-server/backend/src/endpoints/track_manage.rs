use crate::MioState;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use futures::StreamExt;
use log::*;
use mio_common::*;
use serde::Deserialize;
use tokio::fs::{remove_file, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/tu", put(track_upload))
        .route("/td", put(track_delete))
}

// TODO: this doesn't go here
#[derive(Deserialize)]
struct TUQuery {
    fname: Option<String>,
}

async fn track_upload(
    State(state): State<MioState>,
    Extension(key): Extension<mio_entity::user::Model>,
    Query(TUQuery { fname }): Query<TUQuery>,
    mut payload: BodyStream,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // TODO: store the filename for dumping purposes find a unique id for the track
    debug!("PUT /track/tu generating UUID");
    let mut uuid;
    let mut file: File;
    let mut real_fname;
    loop {
        uuid = Uuid::new_v4();
        real_fname = format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid);

        // check if file is already taken
        let check = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(real_fname.clone())
            .await;
        match check {
            Ok(x) => {
                trace!("PUT /track/tu opened file {real_fname}");
                file = x;
                break;
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("PUT /track/tu file already exists");
                    continue;
                }
                error!("PUT /track/tu failed to open file: {err}");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // get original filename
    let orig_filename = sanitize_filename::sanitize(fname.unwrap_or_else(|| {
        trace!("PUT /track/tu generated fname with uuid");
        uuid.to_string()
    }));
    debug!("PUT /track/tu filename and uuid used: \"{orig_filename}\" -> \"{real_fname}\": {uuid}");

    // download the file
    //
    // TODO: filesize limits
    //
    // TODO: maybe don't panic on filesystem errors(?)
    //
    // TODO: make this read base64
    while let Some(chunk) = payload.next().await {
        match chunk {
            Ok(chunk) => {
                debug!("PUT /track/tu {uuid}: writing {} bytes", chunk.len());
                file.write_all(&chunk)
                    .await
                    .expect("Failed to write to file: {}");
            }
            // on err just delete the file
            Err(err) => {
                // delete failed upload, as well as all other uploads per this req
                info!("PUT /track/tu failed upload for {uuid}: {err}");
                trace!("PUT /track/tu flushing {uuid}");
                file.flush()
                    .await
                    .expect("Failed to flush uploaded file: {}");
                drop(file);
                rm_file(uuid).await;
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }
    trace!("PUT /track/tu final flushing {uuid}");
    file.flush()
        .await
        .expect("Failed to flush uploaded file: {}");

    // set off tasks to process files
    state
        .proc_tracks_tx
        .send((uuid, key.id, orig_filename))
        .unwrap();
    Ok((
        StatusCode::OK,
        Json(retstructs::UploadReturn { uuid: vec![uuid] }),
    ))
}

// rm's file when track_upload errors out
async fn rm_file(uuid: Uuid) {
    trace!("RM_FILES deleting {uuid}");
    remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid))
        .await
        .expect("unable to remove file: {}");
}

async fn track_delete(
    State(state): State<MioState>,
    Query(id): Query<msgstructs::DeleteQuery>,
    Extension(userid): Extension<Uuid>,
) -> impl IntoResponse {
    todo!()
}
