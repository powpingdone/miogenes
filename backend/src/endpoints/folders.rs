use super::check_dir_in_data_dir;
use crate::error::MioInnerError;
use crate::MioState;
use crate::DATA_DIR;
use anyhow::anyhow;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::*;
use axum::Router;
#[allow(unused)]
use log::*;
use mio_common::*;
use std::path::PathBuf;
use tokio::fs::create_dir;
use tokio::fs::metadata;
use tokio::fs::rename;
use tokio::fs::try_exists;

// TODO: pepper with log
pub fn routes() -> Router<MioState> {
    Router::new()
        .route("/new", put(folder_create))
        .route("/rename", patch(folder_rename))
        .route("/tree", get(folder_tree))
        .route("/delete", delete(folder_delete))
}

async fn folder_create(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> impl IntoResponse {
    tokio::task::block_in_place(|| check_dir_in_data_dir(format!("{path}/{name}"), userid))?;
    let _hold = state.lock_files.write().await;
    let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &format!("{userid}"), &path, &name]
        .iter()
        .collect();
    create_dir(pbuf).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::AlreadyExists => {
            MioInnerError::Conflict(anyhow!("folder {name} already exists"))
        }
        _ => MioInnerError::from(err),
    })
}

async fn folder_rename(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderRename {
        path,
        old_name,
        new_name,
    }): Query<msgstructs::FolderRename>,
) -> impl IntoResponse {
    // setup vars
    let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &format!("{userid}"), &path]
        .iter()
        .collect();
    let mut old = pbuf.clone();
    old.push(&old_name);
    let old = old.canonicalize()?;
    let mut new = pbuf.clone();
    new.push(&new_name);
    let new = new.canonicalize()?;

    // majority of blocking code here
    tokio::task::block_in_place(|| {
        check_dir_in_data_dir(format!("{path}/{old_name}"), userid)?;
        check_dir_in_data_dir(format!("{path}/{new_name}"), userid)?;

        // make sure it's in the same parent directory
        let real = pbuf.components().collect::<Vec<_>>();
        let oldn = old.components().collect::<Vec<_>>();
        let newn = new.components().collect::<Vec<_>>();
        if &real[..real.len() - 1] != &oldn[..oldn.len() - 1]
            || &real[..real.len() - 1] != &newn[..newn.len() - 1]
        {
            return Err(MioInnerError::ExtIoError(
                anyhow!("the movement will not result in the same directory"),
                StatusCode::BAD_REQUEST,
            ));
        }
        Ok(())
    })?;

    // and make sure it's also a directory
    let _hold = state.lock_files.write().await;
    if !metadata(format!("{path}/{old_name}")).await?.is_dir() {
        return Err(MioInnerError::ExtIoError(
            anyhow!("the directory specified is not a directory"),
            StatusCode::BAD_REQUEST,
        ));
    }

    // and that we're not in conflict
    if !try_exists(format!("{path}/{new_name}"))
        .await
        .is_ok_and(|x| x)
    {
        return Err(MioInnerError::ExtIoError(
            anyhow!("the new directory specified already exists"),
            StatusCode::CONFLICT,
        ));
    }

    // finally, rename
    rename(old, new).await.map_err(MioInnerError::from)
}

async fn folder_tree(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
) -> Result<impl IntoResponse, MioInnerError> {
    Ok(todo!())
}

async fn folder_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> Result<impl IntoResponse, MioInnerError> {
    Ok(todo!())
}
