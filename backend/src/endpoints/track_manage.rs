use std::path::{Path, PathBuf};

use crate::db::write_transaction;
use crate::endpoints::check_dir_in_data_dir;
use crate::error::MioInnerError;
use crate::MioState;
use anyhow::anyhow;
use axum::body::Body;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use futures::StreamExt;
#[allow(unused)]
use log::*;
use mio_common::*;
use tokio::fs::{remove_file, rename, File, OpenOptions};
use tokio::io::{AsyncWriteExt, ErrorKind};
use tokio::time::timeout;
use uuid::Uuid;

// TODO: tests with actual files.
pub fn routes() -> Router<MioState> {
    Router::new().route(
        "/",
        post(track_upload)
            .get(track_stream)
            .patch(track_move)
            .delete(track_delete),
    )
}

#[tracing::instrument]
async fn track_upload(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackUploadQuery { fname, dir }): Query<msgstructs::TrackUploadQuery>,
    payload: Body,
) -> impl IntoResponse {
    let mut payload = payload.into_data_stream();
    trace!("/track/upload acquiring directory lock");
    let _lock_hold = state.lock_files.clone();
    let _hold = _lock_hold.read().await;

    // find a unique id for the track
    debug!("/track/upload generating UUID");
    let mut track_id;
    let mut file: File;
    let mut real_fname;

    // generate filename
    loop {
        track_id = Uuid::new_v4();
        real_fname = crate::DATA_DIR
            .get()
            .unwrap()
            .join(format!("{userid}"))
            .join(&dir)
            .join(format!("{track_id}"));
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
                    "/track/upload opened file {}",
                    real_fname.as_os_str().to_string_lossy()
                );
                file = opened_file;
                break;
            }
            Err(err) => {
                if err.kind() == ErrorKind::AlreadyExists {
                    trace!("/track/upload file already exists");
                    continue;
                }
                error!("/track/upload failed to open file: {err}");
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
        "/track/upload filename and uuid used: \"{orig_filename}\" -> \"{}\": {track_id}",
        real_fname.as_os_str().to_string_lossy()
    );

    // TODO: filesize limits
    //
    // download the file
    loop {
        let chunk = timeout(core::time::Duration::from_secs(10), payload.next()).await;
        match chunk {
            Ok(Some(Ok(chunk))) => {
                if let Err(err) = file.write_all(&chunk).await {
                    error!("/track/upload failed to write to file: {err}");
                    file.flush().await?;
                    drop(file);
                    remove_file(real_fname).await?;
                    return Err(MioInnerError::from(err));
                }
            }
            // end of stream
            Ok(None) => {
                break;
            }
            // on err just delete the file
            Err(_) | Ok(Some(Err(_))) => {
                // extract err
                let err = {
                    if chunk.is_err() {
                        "upload timeout hit".to_string()
                    } else {
                        format!(
                            "chunk streaming error: {}",
                            chunk.unwrap().unwrap().unwrap_err()
                        )
                    }
                };

                // delete failed upload
                error!("/track/upload failure during streaming chunk: {err}");
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
    trace!("/track/upload out of chunks, final flushing {track_id}");
    file.shutdown().await?;

    // set off task to process files
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

#[tracing::instrument]
async fn track_stream(
    State(state): State<MioState>,
    Query(msgstructs::IdInfoQuery { id }): Query<msgstructs::IdInfoQuery>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
) -> impl IntoResponse {
    trace!("/track/stream locking read dir");
    let _hold = state.lock_files.read().await;

    // get dir
    trace!("/track/stream grabbing dir");
    let mut conn = state.db.acquire().await?;
    let dir = sqlx::query!(
        "SELECT path FROM track WHERE id = ? AND owner = ?;",
        id,
        userid
    )
    .fetch_optional(&mut *conn)
    .await?
    .map(|x| x.path)
    .ok_or_else(|| MioInnerError::NotFound(anyhow!("{id} under {userid} does not exist")))?;
    drop(conn);

    // load track into stream
    trace!("/track/stream requesting track {id} under user {userid} via dir {dir}");
    let file = tokio::fs::read(
        crate::DATA_DIR
            .get()
            .unwrap()
            .join(format!("{userid}"))
            .join(&dir)
            .join(format!("{id}")),
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

#[tracing::instrument]
async fn track_move(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::TrackMove { id, new_path }): Query<msgstructs::TrackMove>,
) -> impl IntoResponse {
    trace!("/track/move locking write dir");
    let _hold = state.lock_files.write().await;
    let mut conn = state.db.acquire().await?;
    let new_path = new_path.join("/");
    write_transaction(&mut conn, |txn| {
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

            // curr_fname is not checked as it comes directly from the server
            let curr_fname = crate::DATA_DIR
                .get()
                .unwrap()
                .join(format!("{userid}"))
                .join(&dir)
                .join(format!("{id}"));
            let next_fname = crate::DATA_DIR
                .get()
                .unwrap()
                .join(format!("{userid}"))
                .join(&new_path)
                .join(format!("{id}"));
            check_dir_in_data_dir(next_fname.clone(), userid)?;

            // note: no collision check is needed because every id is almost certainly
            // guaranteed to be unique. begin the actual meat of the transaction
            sqlx::query!(
                "UPDATE track SET path = ? WHERE id = ? AND owner = ?;",
                new_path,
                id,
                userid
            )
            .execute(&mut *txn)
            .await?;
            rename(curr_fname, next_fname).await?;
            Ok::<_, MioInnerError>(StatusCode::OK)
        })
    })
    .await
}

#[tracing::instrument]
async fn track_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::DeleteQuery { id }): Query<msgstructs::DeleteQuery>,
) -> impl IntoResponse {
    trace!("/track/delete locking write dir");
    let _hold = state.lock_files.write().await;
    let mut conn = state.db.acquire().await?;
    write_transaction(&mut conn, |txn| {
        Box::pin(async move {
            // fetch path and delete from db
            trace!("/track/delete finding path to remove");
            let path = sqlx::query!(
                "SELECT path FROM track WHERE id = ? AND owner = ?;",
                id,
                userid
            )
            .fetch_optional(&mut *txn)
            .await?
            .map(|x| x.path)
            .ok_or_else(|| {
                MioInnerError::NotFound(anyhow!("track {id} for owner {userid} does not exist"))
            })?;
            sqlx::query!("DELETE FROM track WHERE id = ? AND owner = ?;", id, userid)
                .execute(&mut *txn)
                .await?;

            // delete realspace file
            let path = crate::DATA_DIR
                .get()
                .unwrap()
                .join(format!("{userid}"))
                .join(&path)
                .join(format!("{id}"));
            trace!("/track/delete path to delete is {path:?}");
            remove_file(path).await?;
            Ok::<_, MioInnerError>(StatusCode::OK)
        })
    })
    .await
}
