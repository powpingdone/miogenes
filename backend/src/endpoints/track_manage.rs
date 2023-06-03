use crate::MioState;
use crate::error::MioInnerError;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use futures::StreamExt;
use log::*;
use mio_common::*;
use std::path::MAIN_SEPARATOR;
use tokio::fs::{
    remove_file,
    File,
    OpenOptions,
};
use tokio::io::{
    AsyncWriteExt,
    ErrorKind,
};
use uuid::Uuid;
use anyhow::anyhow;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/tu", put(track_upload))
        .route("/td", put(track_delete))
        .route("/stream", get(track_stream))
}

async fn track_upload(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackUploadQuery { fname }): Query<msgstructs::TrackUploadQuery>,
    mut payload: BodyStream,
) -> Result<impl IntoResponse, impl IntoResponse> {
    // TODO: store the filename for dumping purposes find a unique id for the track
    debug!("PUT /track/tu generating UUID");
    let mut uuid;
    let mut file: File;
    let mut real_fname;

    // create the dir if not exists
    if let Err(err) =
        tokio::fs::create_dir(format!("{}{MAIN_SEPARATOR}{}", crate::DATA_DIR.get().unwrap(), userid)).await {
        if err.kind() != ErrorKind::AlreadyExists {
            error!("PUT /track/tu failed to create user directory: {err}");
            return Err(
                MioInnerError::TrackProcessingError(
                    anyhow!("failed to begin uploading serverside"),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ),
            );
        }
    }

    // generate filename
    loop {
        uuid = Uuid::new_v4();
        real_fname = format!("{}{MAIN_SEPARATOR}{}{MAIN_SEPARATOR}{}", crate::DATA_DIR.get().unwrap(), userid, uuid);

        // check if file is already taken
        let check = OpenOptions::new().create_new(true).read(true).write(true).open(real_fname.clone()).await;
        match check {
            Ok(x) => {
                trace!("PUT /track/tu opened file {real_fname}");
                file = x;
                break;
            },
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("PUT /track/tu file already exists");
                    continue;
                }
                error!("PUT /track/tu failed to open file: {err}");
                return Err(
                    MioInnerError::TrackProcessingError(
                        anyhow!("failed to read serverside"),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ),
                );
            },
        }
    }

    // get original filename
    let orig_filename = sanitize_filename::sanitize_with_options(fname.unwrap_or_else(|| {
        trace!("PUT /track/tu generated fname with uuid");
        uuid.to_string()
    }), sanitize_filename::Options {
        windows: true,
        ..Default::default()
    });
    debug!("PUT /track/tu filename and uuid used: \"{orig_filename}\" -> \"{real_fname}\": {uuid}");

    // TODO: filesize limits
    //
    // TODO: maybe don't panic on filesystem errors(?)
    //
    // TODO: upload timeout if body stops streaming
    //
    // download the file
    while let Some(chunk) = payload.next().await {
        match chunk {
            Ok(chunk) => {
                if let Err(err) = file.write_all(&chunk).await {
                    error!("PUT /track/tu failed to write to file: {err}");
                    file.flush().await.expect("Failed to flush uploaded file: {}");
                    drop(file);
                    rm_file(uuid).await;
                    return Err(
                        MioInnerError::TrackProcessingError(
                            anyhow!("failed to write serverside"),
                            StatusCode::INTERNAL_SERVER_ERROR,
                        ),
                    );
                }
            },
            // on err just delete the file
            Err(err) => {
                // delete failed upload, as well as all other uploads per this req
                error!("PUT /track/tu failure during streaming chunk: {err}");
                rm_file(uuid).await;
                return Err(
                    MioInnerError::TrackProcessingError(
                        anyhow!("failed to stream chunk: {err}"),
                        StatusCode::BAD_REQUEST,
                    ),
                );
            },
        }
    }
    trace!("PUT /track/tu out of chunks, final flushing {uuid}");
    file.shutdown().await.expect("Failed to shutdown uploaded file: {}");

    // set off tasks to process files
    crate::subtasks::track_upload::track_upload_process(state, uuid, userid, orig_filename).await?;
    Ok((StatusCode::OK, Json(retstructs::UploadReturn { uuid: vec![uuid] })))
}

// rm's file when track_upload errors out
async fn rm_file(uuid: Uuid) {
    trace!("RM_FILES deleting {uuid}");
    remove_file(format!("{}{}", crate::DATA_DIR.get().unwrap(), uuid)).await.expect("unable to remove file: {}");
}

async fn track_stream(
    State(_state): State<MioState>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    // TODO: transcode into something browser friendly, as the file on disk may not
    // actually be consumable by the browser
    //
    // TODO: possibly make this a body stream? errors _will_ be an issue...
    trace!("GET /track/stream requesting track {id} under user {userid}");

    // read in file
    let file =
        tokio::fs::read(
            format!("{}{MAIN_SEPARATOR}{}{MAIN_SEPARATOR}{}", crate::DATA_DIR.get().unwrap(), userid, id),
        ).await;
    if let Err(err) = file {
        return if err.kind() == ErrorKind::NotFound {
            debug!("GET /track/stream track {id} under user {userid} doesn't exist");
            Err(StatusCode::NOT_FOUND)
        } else {
            error!("GET /track/stream error encountered while opening file: {err}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        };
    }
    trace!("GET /track/stream sending back {} bytes from {id}", file.as_ref().unwrap().len());
    Ok((StatusCode::OK, file.unwrap()))
}

async fn track_delete(
    State(state): State<MioState>,
    Query(id): Query<msgstructs::DeleteQuery>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    todo!()
}
