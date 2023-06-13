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
use path_absolutize::Absolutize;
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

// TODO: join! dir locking and check_dir_in..

async fn folder_create(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> impl IntoResponse {
    let name = sanitize_filename::sanitize(name);
    let _hold = state.lock_files.write().await;
    tokio::task::spawn_blocking({
        let pbuf = [&path].iter().collect::<PathBuf>();
        move || check_dir_in_data_dir(&pbuf, userid)
    })
    .await??;
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
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderRename { old_path, new_path }): Query<msgstructs::FolderRename>,
) -> impl IntoResponse {
    // setup vars
    let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &format!("{userid}")]
        .iter()
        .collect();
    let mut old = pbuf.clone();
    old.push(&old_path);
    let old = old.absolutize()?;
    let mut new = pbuf.clone();
    new.push(&new_path);
    let new = new.absolutize()?;
    debug!(
        "PATCH /api/folder trying to move \"{}\" -> \"{}\"",
        old.display(),
        new.display()
    );

    // majority of blocking code here
    tokio::task::spawn_blocking({
        let old_path = old_path.clone();
        let new_path = new_path.clone();
        move || {
            check_dir_in_data_dir(&old_path, userid)?;
            check_dir_in_data_dir(&new_path, userid)
        }
    })
    .await??;

    // make sure it exists
    let _hold = state.lock_files.write().await;
    if !try_exists(&old).await? {
        return Err(MioInnerError::NotFound(anyhow!("{old_path}")));
    }

    // and make sure it's also a directory
    if !metadata(&old).await?.is_dir() {
        return Err(MioInnerError::ExtIoError(
            anyhow!("the directory specified is not a directory"),
            StatusCode::BAD_REQUEST,
        ));
    }

    // and that we're not in conflict
    if try_exists(&new).await? {
        return Err(MioInnerError::Conflict(anyhow!("{new_path}")));
    }

    // finally, rename
    rename(old, new).await.map_err(MioInnerError::from)
}

async fn folder_query(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
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
                folder_tree_inner(top_level.clone(), read_dir(top_level).await?, "".into()).await?,
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
        debug!("FTI call to ({base_dir:?}, {curr_path:?})");
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

        // 3: flatten. the current path is appended too
        let mut ret = vec![curr_path.clone()];
        ret.extend(dirs_finished.into_iter().flatten());
        Ok(ret)
    })
}

async fn folder_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
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

#[cfg(test)]
mod test {
    use std::{collections::HashSet, path::PathBuf};

    use crate::test::*;
    use axum::http::Method;
    use mio_common::{msgstructs, retstructs};

    #[tokio::test]
    async fn folder_good() {
        let cli = client().await;
        let jwt = gen_user(&cli, "folder_good").await;
        // create folders
        jwt_header(
            &cli,
            Method::PUT,
            &format!(
                "/api/folder?{}",
                serde_urlencoded::to_string(&msgstructs::FolderCreateDelete {
                    name: "a horse".to_string(),
                    path: "".to_string()
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        jwt_header(
            &cli,
            Method::PUT,
            &format!(
                "/api/folder?{}",
                serde_urlencoded::to_string(&msgstructs::FolderCreateDelete {
                    name: "neigh".to_string(),
                    path: "a horse/".to_string()
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        jwt_header(
            &cli,
            Method::PUT,
            &format!(
                "/api/folder?{}",
                serde_urlencoded::to_string(&msgstructs::FolderCreateDelete {
                    name: "bleh".to_string(),
                    path: "a horse/neigh".to_string()
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;

        // get tree
        let ret = jwt_header(&cli, Method::GET, "/api/folder", &jwt)
            .await
            .json::<retstructs::FolderQuery>()
            .ret;
        if let retstructs::FolderQueryRet::Tree(ret) = ret {
            let ret = ret.into_iter().collect::<HashSet<_>>();
            assert_eq!(
                ret,
                HashSet::from_iter(
                    ["", "a horse", "a horse/neigh", "a horse/neigh/bleh"]
                        .into_iter()
                        .map(PathBuf::from)
                )
            );
        } else {
            panic!("did not return tree when supposted to");
        }
        // rename
        jwt_header(
            &cli,
            Method::PATCH,
            &format!(
                "/api/folder?{}",
                serde_urlencoded::to_string(&msgstructs::FolderRename {
                    old_path: "a horse".to_string(),
                    new_path: "merasmus".to_string()
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        // check tree again
        let ret = jwt_header(&cli, Method::GET, "/api/folder", &jwt)
            .await
            .json::<retstructs::FolderQuery>()
            .ret;
        if let retstructs::FolderQueryRet::Tree(ret) = ret {
            let ret = ret.into_iter().collect::<HashSet<_>>();
            assert_eq!(
                ret,
                HashSet::from_iter(
                    ["", "merasmus", "merasmus/neigh", "merasmus/neigh/bleh"]
                        .into_iter()
                        .map(PathBuf::from)
                )
            );
        } else {
            panic!("did not return tree when supposted to");
        }
        // delete
        todo!()
    }

    #[tokio::test]
    async fn folder_bad_path_checks() {}
}
