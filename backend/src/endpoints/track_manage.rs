use crate::endpoints::check_dir_in_data_dir;
use crate::error::MioInnerError;
use crate::MioState;
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use futures::StreamExt;
#[allow(unused)]
use log::*;
use mio_common::*;
use std::path::PathBuf;
use tokio::fs::{remove_file, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/upload", put(track_upload))
        .route("/move", patch(track_move))
        .route("/delete", delete(track_delete))
        .route("/stream", get(track_stream))
}

async fn track_upload(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackUploadQuery { fname, dir }): Query<msgstructs::TrackUploadQuery>,
    mut payload: BodyStream,
) -> impl IntoResponse {
    trace!("PUT /track/upload acquiring directory lock");
    let _lock_hold = state.lock_files.clone();
    let _hold = _lock_hold.write().await;

    // TODO: store the filename for dumping purposes
    //
    // find a unique id for the track
    debug!("PUT /track/upload generating UUID");
    let mut track_id;
    let mut file: File;
    let mut real_fname;

    // generate filename
    loop {
        track_id = Uuid::new_v4();
        real_fname = [
            *crate::DATA_DIR.get().unwrap(),
            &format!("{userid}"),
            dir.as_ref(),
            &format!("{track_id}"),
        ]
        .into_iter()
        .collect::<PathBuf>();
        tokio::task::block_in_place(|| check_dir_in_data_dir(real_fname.clone(), userid))?;

        // check if file is already taken
        let check = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(real_fname.clone())
            .await;
        match check {
            Ok(opened_file) => {
                trace!(
                    "PUT /track/upload opened file {}",
                    real_fname.as_os_str().to_string_lossy()
                );
                file = opened_file;
                break;
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("PUT /track/upload file already exists");
                    continue;
                }
                error!("PUT /track/upload failed to open file: {err}");
                return Err(MioInnerError::from(err));
            }
        }
    }

    // get original filename
    let orig_filename = sanitize_filename::sanitize_with_options(
        fname.unwrap_or_else(|| {
            trace!("PUT /track/upload generated fname with uuid");
            track_id.to_string()
        }),
        sanitize_filename::Options {
            windows: true,
            ..Default::default()
        },
    );
    debug!(
        "PUT /track/upload filename and uuid used: \"{orig_filename}\" -> \"{}\": {track_id}",
        real_fname.as_os_str().to_string_lossy()
    );

    // TODO: filesize limits
    //
    // TODO: upload timeout if body stops streaming
    //
    // download the file
    while let Some(chunk) = payload.next().await {
        match chunk {
            Ok(chunk) => {
                if let Err(err) = file.write_all(&chunk).await {
                    error!("PUT /track/upload failed to write to file: {err}");
                    file.flush().await?;
                    drop(file);
                    remove_file(real_fname).await?;
                    return Err(MioInnerError::from(err));
                }
            }
            // on err just delete the file
            Err(err) => {
                // delete failed upload, as well as all other uploads per this req
                error!("PUT /track/upload failure during streaming chunk: {err}");
                file.flush().await?;
                drop(file);
                remove_file(real_fname).await?;
                return Err(MioInnerError::TrackProcessingError(
                    anyhow!("failed to stream chunk: {err}"),
                    StatusCode::BAD_REQUEST,
                ));
            }
        }
    }
    trace!("PUT /track/upload, out of chunks, final flushing {track_id}");
    file.shutdown()
        .await
        .expect("Failed to shutdown uploaded file: {}");

    // set off tasks to process files
    crate::subtasks::track_upload::track_upload_process(
        state,
        track_id,
        real_fname,
        dir,
        userid,
        orig_filename,
    )
    .await?;
    Ok((
        StatusCode::OK,
        Json(retstructs::UploadReturn { uuid: track_id }),
    ))
}

async fn track_stream(
    State(state): State<MioState>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    trace!("GET /track/stream locking read dir");
    let _hold = state.lock_files.read().await;

    // get dir
    trace!("GET /track/stream grabbing dir");
    let mut conn = state.db.acquire().await?;
    let Some(
        dir
    ) = sqlx::query!("SELECT path FROM track WHERE id = ?;", id).fetch_optional(&mut *conn).await?.map(|x| x.path)
    else {
        return Err(MioInnerError::NotFound(anyhow!("{id} under {userid} does not exist")));
    };
    drop(conn);

    // TODO: transcode into something browser friendly, as the file on disk may not
    // actually be consumable by the browser
    // read in file
    trace!("GET /track/stream requesting track {id} under user {userid} via dir {dir}");
    let file = tokio::fs::read(
        [
            *crate::DATA_DIR.get().unwrap(),
            &format!("{userid}"),
            &dir,
            &format!("{id}"),
        ]
        .into_iter()
        .collect::<PathBuf>(),
    )
    .await;
    match file {
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                debug!("GET /track/stream track {id} under user {userid} doesn't exist");

                // TODO: maybe delete the id from the table
                Err(MioInnerError::NotFound(anyhow!(
                    "{id} under {userid} does not exist, despite the db saying it exists."
                )))
            } else {
                Err(MioInnerError::from(err))
            }
        }
        Ok(bytes) => {
            trace!(
                "GET /track/stream sending back {:?} bytes from {id}",
                bytes.len()
            );
            Ok((StatusCode::OK, bytes))
        }
    }
}

async fn track_move(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackMove { id, new_path }): Query<msgstructs::TrackMove>,
) -> impl IntoResponse {
    todo!()
}

async fn track_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::DeleteQuery { id }): Query<msgstructs::DeleteQuery>,
) -> impl IntoResponse {
    todo!()
}
