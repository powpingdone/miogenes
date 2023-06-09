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
use futures::Future;
#[allow(unused)]
use log::*;
use mio_common::*;
use std::path::PathBuf;
use std::path::MAIN_SEPARATOR_STR;
use std::pin::Pin;
use tokio::fs::create_dir;
use tokio::fs::metadata;
use tokio::fs::read_dir;
use tokio::fs::remove_dir;
use tokio::fs::rename;
use tokio::fs::try_exists;
use tokio::fs::ReadDir;
use uuid::Uuid;

// TODO: pepper with log
pub fn routes() -> Router<MioState> {
    Router::new().route(
        "/",
        put(folder_create)
            .get(folder_query)
            .patch(folder_rename)
            .delete(folder_delete),
    )
}

async fn folder_create(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> impl IntoResponse {
    check_dir_in_data_dir(format!("{path}/{name}"), userid)?;
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
    tokio::task::spawn_blocking({
        let path = path.clone();
        let old_name = old_name.clone();
        let new_name = new_name.clone();
        let old = old.clone();
        let new = new.clone();
        move || {
            check_dir_in_data_dir(format!("{path}/{old_name}"), userid)?;
            check_dir_in_data_dir(format!("{path}/{new_name}"), userid)?;

            // make sure it's in the same parent directory
            let real = pbuf.components().collect::<Vec<_>>();
            let oldn = old.components().collect::<Vec<_>>();
            let newn = new.components().collect::<Vec<_>>();
            if real[..real.len() - 1] != oldn[..oldn.len() - 1]
                || real[..real.len() - 1] != newn[..newn.len() - 1]
            {
                return Err(MioInnerError::ExtIoError(
                    anyhow!("the movement will not result in the same directory"),
                    StatusCode::BAD_REQUEST,
                ));
            }
            Ok(())
        }
    })
    .await??;

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
        return Err(MioInnerError::Conflict(anyhow!(
            "the new directory specified already exists"
        )));
    }

    // finally, rename
    rename(old, new).await.map_err(MioInnerError::from)
}

async fn folder_query(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    path_query: Option<Query<msgstructs::FolderQuery>>,
) -> Result<impl IntoResponse, MioInnerError> {
    let _hold = state.lock_files.read().await;
    let top_level = [*crate::DATA_DIR.get().unwrap(), &format!("{userid}")]
        .into_iter()
        .collect::<PathBuf>();
    if let Some(Query(msgstructs::FolderQuery { path })) = path_query {
        // query individual folder
        let mut check = top_level.clone();
        check.push(path);
        check_dir_in_data_dir(check.clone(), userid)?;
        let mut ret = vec![];
        let mut read_dir = read_dir(check).await?;
        while let Some(x) = read_dir.next_entry().await? {
            if x.file_type().await?.is_file() {
                ret.push(
                    Uuid::parse_str(
                        x.file_name().into_string().map_err(|err| {
                            MioInnerError::IntIoError(
                                anyhow!(
                                    "could not convert internal file name into string to become uuid: name is {err:?}"
                                ),
                            )
                        })?.as_str(),
                    ).map_err(|err| MioInnerError::IntIoError(anyhow!("internal file name is not a uuid: {err}")))?,
                );
            }
        }
        Ok(Json(retstructs::FolderQuery {
            ret: retstructs::FolderQueryRet::Track(ret),
        }))
    } else {
        // query folder tree
        Ok(Json(retstructs::FolderQuery {
            ret: retstructs::FolderQueryRet::Tree(
                folder_tree_inner(
                    top_level.clone(),
                    read_dir(top_level).await?,
                    MAIN_SEPARATOR_STR.try_into().unwrap(),
                )
                .await?,
            ),
        }))
    }
}

// TODO: TEST THIS FOR THE LOVE OF HOLY MOTHER MARY JOESPH
fn folder_tree_inner(
    base_dir: PathBuf,
    mut dir: ReadDir,
    curr_path: PathBuf,
) -> Pin<Box<dyn Future<Output = Result<Vec<PathBuf>, MioInnerError>> + Send>> {
    // The strange return type is due to recursion. I don't think that this could have
    // been impl'd reasonably any other way. Thanks rust async!
    Box::pin(async move {
        // 1: collect all the dirs
        let mut dirs = vec![];
        while let Some(file) = dir.next_entry().await? {
            if file.file_type().await?.is_dir() {
                dirs.push(file.file_name());
            }
        }

        // 2: for each dir, recurse into each and see if they have any dirs
        let dir_futs = dirs
            .into_iter()
            .map(|next_dir| {
                let next_display_path = [curr_path.clone(), next_dir.into()]
                    .into_iter()
                    .collect::<PathBuf>();
                let reading_dir = [base_dir.clone(), next_display_path.clone()]
                    .into_iter()
                    .collect::<PathBuf>();
                tokio::spawn({
                    let base_dir = base_dir.clone();
                    async move {
                        folder_tree_inner(base_dir, read_dir(reading_dir).await?, next_display_path)
                            .await
                    }
                })
            })
            .collect::<Vec<_>>();
        let mut dirs_finished = vec![];
        for fut in dir_futs {
            dirs_finished.push(fut.await??);
        }

        // 3: flatten, and concatenate with the curr path. the current path is appended too
        let mut ret = vec![curr_path.clone()];
        ret.extend(
            dirs_finished
                .into_iter()
                .flatten()
                .map(|x| [curr_path.clone(), x].into_iter().collect()),
        );
        Ok(ret)
    })
}

async fn folder_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> Result<impl IntoResponse, MioInnerError> {
    let _hold = state.lock_files.write().await;
    let real_path = [
        *crate::DATA_DIR.get().unwrap(),
        &format!("{userid}"),
        &path,
        &name,
    ]
    .into_iter()
    .collect::<PathBuf>();
    check_dir_in_data_dir(&real_path, userid)?;

    // check if dir has contents https://github.com/rust-lang/rust/issues/86442
    if read_dir(&real_path).await?.next_entry().await?.is_none() {
        return Err(MioInnerError::ExtIoError(
            anyhow!(
                "Directory {:?} has items, please remove them.",
                [name, path].into_iter().collect::<PathBuf>()
            ),
            StatusCode::BAD_REQUEST,
        ));
    }
    remove_dir(real_path).await?;
    Ok(())
}
