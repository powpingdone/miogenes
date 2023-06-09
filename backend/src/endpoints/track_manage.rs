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
use sqlx::Connection;
use std::path::PathBuf;
use tokio::fs::{remove_file, rename, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use uuid::Uuid;

pub fn routes() -> Router<MioState> {
    Router::new().route(
        "/",
        post(track_upload)
            .get(track_stream)
            .patch(track_move)
            .delete(track_delete),
    )
}

async fn track_upload(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackUploadQuery { fname, dir }): Query<msgstructs::TrackUploadQuery>,
    mut payload: BodyStream,
) -> impl IntoResponse {
    trace!("track_upload acquiring directory lock");
    let _lock_hold = state.lock_files.clone();
    let _hold = _lock_hold.write().await;

    // TODO: store the filename for dumping purposes
    //
    // find a unique id for the track
    debug!("track_upload generating UUID");
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
        check_dir_in_data_dir(real_fname.clone(), userid)?;

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
                    "track_upload opened file {}",
                    real_fname.as_os_str().to_string_lossy()
                );
                file = opened_file;
                break;
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("track_upload file already exists");
                    continue;
                }
                error!("track_upload failed to open file: {err}");
                return Err(MioInnerError::from(err));
            }
        }
    }

    // get original filename
    let orig_filename = sanitize_filename::sanitize_with_options(
        fname.unwrap_or_else(|| {
            trace!("PUT track_upload generated fname with uuid");
            track_id.to_string()
        }),
        sanitize_filename::Options {
            windows: true,
            ..Default::default()
        },
    );
    debug!(
        "track_upload filename and uuid used: \"{orig_filename}\" -> \"{}\": {track_id}",
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
                    error!("PUT track_upload failed to write to file: {err}");
                    file.flush().await?;
                    drop(file);
                    remove_file(real_fname).await?;
                    return Err(MioInnerError::from(err));
                }
            }
            // on err just delete the file
            Err(err) => {
                // delete failed upload, as well as all other uploads per this req
                error!("track_upload failure during streaming chunk: {err}");
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
    trace!("track_upload, out of chunks, final flushing {track_id}");
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
    trace!("/track/stream locking read dir");
    let _hold = state.lock_files.read().await;

    // get dir
    trace!("/track/stream grabbing dir");
    let mut conn = state.db.acquire().await?;
    let Some(
        dir
    ) = sqlx:: query !(
        "SELECT path FROM track WHERE id = ? AND owner = ?;",
        id,
        userid
    ).fetch_optional(&mut *conn).await ?.map(|x| x.path) else {
        return Err(MioInnerError::NotFound(anyhow!("{id} under {userid} does not exist")));
    };
    drop(conn);

    // TODO: transcode into something browser friendly, as the file on disk may not
    // actually be consumable by the browser read in file
    trace!("/track/stream requesting track {id} under user {userid} via dir {dir}");
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
                warn!("/track/stream track {id} under user {userid} doesn't exist, despite the db saying it does.");

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
                "/track/stream sending back {:?} bytes from {id}",
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
    trace!("/track/move locking read dir");
    let _hold = state.lock_files.write().await;
    state
        .db
        .acquire()
        .await?
        .transaction(|txn| {
            Box::pin(async move {
                // preliminary checks
                let dir = sqlx::query!(
                    "SELECT path FROM track WHERE id = ? AND owner = ?;",
                    id,
                    userid
                )
                .fetch_optional(&mut *txn)
                .await?
                .map(|x| x.path)
                .ok_or_else(|| {
                    MioInnerError::NotFound(anyhow!("could not find id {id} for user {userid}"))
                })?;
                let curr_fname = [
                    *crate::DATA_DIR.get().unwrap(),
                    &format!("{userid}"),
                    dir.as_ref(),
                    &format!("{id}"),
                ]
                .into_iter()
                .collect::<PathBuf>();
                let next_fname = [
                    *crate::DATA_DIR.get().unwrap(),
                    &format!("{userid}"),
                    new_path.as_ref(),
                    &format!("{id}"),
                ]
                .into_iter()
                .collect::<PathBuf>();
                check_dir_in_data_dir(next_fname.clone(), userid)?;

                // note: no collision check is needed because every id is guaranteed to be unique.
                // begin the actual meat of the transaction
                rename(curr_fname, next_fname).await?;
                sqlx::query!(
                    "UPDATE track SET path = ? WHERE id = ? AND owner = ?;",
                    new_path,
                    id,
                    userid
                )
                .execute(&mut *txn)
                .await?;
                Ok::<_, MioInnerError>(StatusCode::OK)
            })
        })
        .await
}

async fn track_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::DeleteQuery { id }): Query<msgstructs::DeleteQuery>,
) -> impl IntoResponse {
    trace!("/track/delete locking write dir");
    let _hold = state.lock_files.write().await;
    state.db.acquire().await?.transaction(|txn| Box::pin(async move {
        let Some(
            path
        ) = sqlx:: query !(
            "SELECT path FROM track WHERE id = ? AND owner = ?;",
            id,
            userid
        ).fetch_optional(&mut *txn).await ?.map(|x| x.path) else {
            return Err(MioInnerError::NotFound(anyhow!("track {id} for owner {userid} does not exist")));
        };
        let path =
            [*crate::DATA_DIR.get().unwrap(), &format!("{userid}"), &path, &format!("{id}")]
                .into_iter()
                .collect::<PathBuf>();
        sqlx::query!("DELETE FROM track WHERE id = ? AND owner = ?;", id, userid).execute(&mut *txn).await?;
        remove_file(path).await?;
        Ok::<_, MioInnerError>(StatusCode::OK)
    })).await
}
