use axum::Router;
use axum::extract::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use mio_common::*;
use axum::routing::*;
use tokio::fs::create_dir;
use uuid::Uuid;
use crate::DATA_DIR;
use crate::MioState;
use crate::error::MioInnerError;
use std::path::Path;
use std::path::{
    PathBuf,
};
use anyhow::anyhow;

// TODO: pepper with log
pub fn routes() -> Router<MioState> {
    Router::new().route("/new", put(folder_create)).route("/rename", patch(folder_rename))
}

fn check_dir_in_data_dir(path: impl AsRef<Path>, userid: Uuid) -> Result<(), MioInnerError> {
    let real_path = Path::new(&format!("{}/{userid}/", DATA_DIR.get().unwrap())).to_path_buf();
    let mut ask_path = real_path.clone();
    ask_path.push(&path);
    let real_path = real_path.canonicalize().unwrap();
    let ask_path = match ask_path.canonicalize() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(
                MioInnerError::ExtIoError(
                    anyhow!("the path supplied could not be canonicalize'd: {err}"),
                    StatusCode::BAD_REQUEST,
                ),
            );
        },
    };
    let real_path = real_path.components().collect::<Vec<_>>();
    let ask_path = ask_path.components().collect::<Vec<_>>();

    // NOTE: this length check is used to make sure that `zip` in the following for
    // loop does not run out and produce an Ok when the ask path is shorter than the
    // real path.
    if ask_path.len() <= real_path.len() {
        return Err(
            MioInnerError::ExtIoError(
                anyhow!("invalid path: {:?}", path.as_ref().as_os_str()),
                StatusCode::BAD_REQUEST,
            ),
        );
    }
    for (real, ask) in real_path.iter().zip(ask_path.iter()) {
        if real != ask {
            return Err(
                MioInnerError::ExtIoError(
                    anyhow!("invalid path: {:?}", path.as_ref().as_os_str()),
                    StatusCode::BAD_REQUEST,
                ),
            );
        }
    }
    Ok(())
}

async fn folder_create(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreate { name, path }): Query<msgstructs::FolderCreate>,
) -> Result<impl IntoResponse, MioInnerError> {
    tokio::task::block_in_place(|| check_dir_in_data_dir(format!("{path}/{name}"), userid))?;
    let _hold = state.lock_files.write().await;
    let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &path, &name].iter().collect();
    create_dir(pbuf).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::AlreadyExists => {
            MioInnerError::Conflict(anyhow!("folder {name} already exists"))
        },
        _ => {
            MioInnerError::from(err)
        },
    })
}

async fn folder_rename(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderRename { path, old_name, new_name }): Query<msgstructs::FolderRename>,
) -> Result<impl IntoResponse, MioInnerError> {
    tokio::task::block_in_place(|| {
        check_dir_in_data_dir(format!("{path}/{old_name}"), userid)?;
        check_dir_in_data_dir(format!("{path}/{new_name}"), userid)
    })?;
    let _hold = state.lock_files.write().await;
    // make sure it's in the same parent directory
    todo!();
    // and make sure it's also a directory
    todo!();
    // and that we're not in conflict
    todo!();
    Ok(todo!())
}
