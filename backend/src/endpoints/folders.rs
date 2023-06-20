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
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> impl IntoResponse {
    debug!("PUT /api/folder creating folder {name} at {path}");
    let name = sanitize_filename::sanitize(name);
    let (_hold, task) = tokio::join!(
        state.lock_files.write(),
        tokio::task::spawn_blocking({
            let pbuf = [&path].iter().collect::<PathBuf>();
            move || check_dir_in_data_dir(&pbuf, userid)
        })
    );
    task??;
    let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &format!("{userid}"), &path, &name]
        .iter()
        .collect();
    create_dir(pbuf).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::AlreadyExists => MioInnerError::Conflict(anyhow!("{name}")),
        _ => MioInnerError::from(err),
    })
}

async fn folder_rename(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderRename { old_path, new_path }): Query<msgstructs::FolderRename>,
) -> impl IntoResponse {
    debug!("PATCH /api/folder move from '{old_path}' -> '{new_path}'");
    let (_hold, check_dir) = tokio::join!(
        state.lock_files.write(),
        // setup vars, majority of blocking code here
        tokio::task::spawn_blocking({
            let old_path = old_path.clone();
            let new_path = new_path.clone();
            move || -> Result<_, MioInnerError> {
                check_dir_in_data_dir(&old_path, userid)?;
                check_dir_in_data_dir(&new_path, userid)?;

                // generate paths
                let pbuf: PathBuf = [*DATA_DIR.get().unwrap(), &format!("{userid}")]
                    .iter()
                    .collect();
                let mut old = pbuf.clone();
                old.push(&old_path);
                let old = old.absolutize()?.into_owned();
                let mut new = pbuf;
                new.push(&new_path);
                let new = new.absolutize()?.into_owned();
                debug!(
                    "PATCH /api/folder confirmed trying to move \"{}\" -> \"{}\"",
                    old.display(),
                    new.display()
                );
                Ok((old, new))
            }
        })
    );
    let (old, new) = check_dir??;

    // make sure it exists
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
    debug!(
        "PATCH /api/folder finally attempting to move \"{}\" -> \"{}\"",
        old.display(),
        new.display()
    );
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
        debug!("GET /api/folder querying folder {}, {path}", top_level.display());
        let path = AsRef::<std::path::Path>::as_ref(&path).absolutize()?;
        check_dir_in_data_dir(&path, userid)?;
        let mut check = top_level.clone();
        check.push(path);
        let mut ret = vec![];
        let mut read_dir = read_dir(check).await?;
        while let Some(x) = read_dir.next_entry().await? {
            let loghold = x.file_name();
            let logfile = AsRef::<std::path::Path>::as_ref(&loghold).display();
            trace!("GET /api/folder checking branch {logfile}");
            if x.file_type().await?.is_file() {
                trace!("GET /api/folder branch is file {logfile}");
                ret.push(
                    Uuid::try_parse(
                        x.file_name().into_string().map_err(|err| {
                            MioInnerError::IntIoError(
                                anyhow!(
                                    "could not convert internal file name into string to become uuid: name is {err:?}"
                                ),
                            )
                        })?.as_str(),
                    ).map(|x| x.to_string()).map_err(|err| MioInnerError::IntIoError(anyhow!("internal file name is not a uuid: {err}")))?,
                );
            }
        }
        Ok(Json(retstructs::FolderQuery { ret }))
    } else {
        // query folder tree
        debug!("GET /api/folder querying folder tree");
        Ok(Json(retstructs::FolderQuery {
            ret: folder_tree_inner(top_level.clone(), read_dir(top_level).await?, "".into())
                .await?,
        }))
    }
}

fn folder_tree_inner(
    base_dir: PathBuf,
    mut dir: ReadDir,
    curr_path: PathBuf,
) -> Pin<Box<dyn Future<Output = Result<Vec<String>, MioInnerError>> + Send>> {
    // The strange return type is due to recursion. I don't think that this could have
    // been impl'd reasonably any other way. Thanks rust async!
    Box::pin(async move {
        debug!("FTI call to ({base_dir:?}, {curr_path:?})");

        fn fail_to_utf8(file: impl AsRef<std::path::Path>) -> MioInnerError {
            MioInnerError::IntIoError(anyhow!(
                "the file \"{}\" could not be converted to a string",
                file.as_ref().display()
            ))
        }

        // 1: collect all the dirs
        let mut dirs = vec![];
        while let Some(file) = dir.next_entry().await? {
            if file.file_type().await?.is_dir() {
                dirs.push({
                    let fname = file.file_name();
                    fname
                        .to_str()
                        .map(|x| x.to_owned())
                        .ok_or_else(|| fail_to_utf8(file.file_name()))?
                });
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
        let mut ret: Vec<String> = vec![curr_path
            .clone()
            .to_str()
            .map(|x| x.to_owned())
            .ok_or_else(|| fail_to_utf8(curr_path))?];
        ret.extend(dirs_finished.into_iter().flatten());
        Ok(ret)
    })
}

async fn folder_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> Result<impl IntoResponse, MioInnerError> {
    let real_path = [
        *crate::DATA_DIR.get().unwrap(),
        &format!("{userid}"),
        &path,
        &name,
    ]
    .into_iter()
    .collect::<PathBuf>();
    debug!(
        "DELETE /api/folder attemping to delete folder {}",
        real_path.display()
    );
    let (_hold, ret) = tokio::join!(
        state.lock_files.write(),
        tokio::task::spawn_blocking({
            let path = path.clone();
            move || check_dir_in_data_dir(&path, userid)
        })
    );
    ret??;

    // check if dir has contents https://github.com/rust-lang/rust/issues/86442
    if read_dir(&real_path).await?.next_entry().await?.is_some() {
        return Err(MioInnerError::ExtIoError(
            anyhow!(
                "Directory {:?} has items, please remove them.",
                [path, name].into_iter().collect::<PathBuf>()
            ),
            StatusCode::BAD_REQUEST,
        ));
    }
    debug!("DELETE /api/folder deleting folder {}", real_path.display());
    remove_dir(real_path).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{error::ErrorMsg, test::*};
    use axum::http::{Method, StatusCode};
    use mio_common::*;
    use serde_urlencoded::to_string as url_enc;
    use std::collections::HashSet;

    // util function to check if dirs are the same
    #[must_use]
    async fn tree_check(cli: &axum_test::TestServer, jwt: &auth::JWT, dirs: &[&str]) -> bool {
        let ret = jwt_header(cli, Method::GET, "/api/folder", jwt)
            .await
            .json::<retstructs::FolderQuery>()
            .ret;

        let ret = ret.into_iter().collect::<HashSet<_>>();
        ret == HashSet::from_iter(dirs.iter().map(|x| x.to_string()))
    }

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
                url_enc(msgstructs::FolderCreateDelete {
                    name: "a horse".to_string(),
                    path: "".to_string(),
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
                url_enc(msgstructs::FolderCreateDelete {
                    name: "neigh".to_string(),
                    path: "a horse/".to_string(),
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
                url_enc(msgstructs::FolderCreateDelete {
                    name: "bleh".to_string(),
                    path: "a horse/neigh".to_string(),
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;

        // get tree
        assert!(
            tree_check(
                &cli,
                &jwt,
                &["", "a horse", "a horse/neigh", "a horse/neigh/bleh"]
            )
            .await
        );

        // rename
        jwt_header(
            &cli,
            Method::PATCH,
            &format!(
                "/api/folder?{}",
                url_enc(msgstructs::FolderRename {
                    old_path: "a horse".to_string(),
                    new_path: "merasmus".to_string(),
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        assert!(
            tree_check(
                &cli,
                &jwt,
                &["", "merasmus", "merasmus/neigh", "merasmus/neigh/bleh"]
            )
            .await
        );

        // move
        jwt_header(
            &cli,
            Method::PATCH,
            &format!(
                "/api/folder?{}",
                url_enc(msgstructs::FolderRename {
                    old_path: "merasmus/neigh/bleh".to_string(),
                    new_path: "merasmus/bleh".to_string(),
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        assert!(
            tree_check(
                &cli,
                &jwt,
                &["", "merasmus", "merasmus/neigh", "merasmus/bleh"]
            )
            .await
        );

        // delete
        jwt_header(
            &cli,
            Method::DELETE,
            "/api/folder?name=bleh&path=merasmus",
            &jwt,
        )
        .await;
        assert!(tree_check(&cli, &jwt, &["", "merasmus", "merasmus/neigh"]).await);
    }

    #[tokio::test]
    #[ignore]
    // this test _MAY MODIFY FILES ON YOUR HARD DRIVE NOT IN THE TEST DIR_, please
    // only run as a verification of your changes
    async fn folder_bad_path_checks() {
        let cli = client().await;
        let jwt = gen_user(&cli, "folder_bad_path_checks").await;
        const TEST_PATHS: &[&str] = &["..", "../", "a/../..", "../../test_files"];
        let err = crate::ErrorMsg {
            error: crate::MioInnerError::ExtIoError(
                anyhow::anyhow!("bad path"),
                StatusCode::BAD_REQUEST,
            )
            .msg(),
        };
        for path in TEST_PATHS {
            for name in TEST_PATHS {
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::PUT,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderCreateDelete {
                                name: name.to_string(),
                                path: path.to_string(),
                            })
                            .unwrap()
                        ),
                        &jwt
                    )
                    .expect_failure()
                    .await
                    .json::<ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::GET,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderQuery {
                                path: path.to_string()
                            })
                            .unwrap()
                        ),
                        &jwt,
                    )
                    .expect_failure()
                    .await
                    .json::<ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::PATCH,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderRename {
                                old_path: path.to_string(),
                                new_path: name.to_string(),
                            })
                            .unwrap()
                        ),
                        &jwt
                    )
                    .expect_failure()
                    .await
                    .json::<ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::DELETE,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderCreateDelete {
                                name: name.to_string(),
                                path: path.to_string(),
                            })
                            .unwrap()
                        ),
                        &jwt,
                    )
                    .expect_failure()
                    .await
                    .json::<ErrorMsg>(),
                    err
                );
            }
        }
    }

    #[tokio::test]
    async fn folder_bad_collison_checks() {
        let cli = client().await;
        let jwt = gen_user(&cli, "folder_bad_collison_checks").await;
        {
            jwt_header(&cli, Method::PUT, "/api/folder?name=a&path=", &jwt).await;
            jwt_header(&cli, Method::PUT, "/api/folder?name=b&path=a", &jwt).await;
            jwt_header(&cli, Method::PUT, "/api/folder?name=1&path=", &jwt).await;
            assert!(tree_check(&cli, &jwt, &["", "a", "a/b", "1"]).await)
        }
        jwt_header(&cli, Method::PUT, "/api/folder?name=b&path=a", &jwt)
            .expect_failure()
            .await;
        jwt_header(
            &cli,
            Method::PATCH,
            "/api/folder?old_path=a&new_path=1",
            &jwt,
        )
        .expect_failure()
        .await;
    }
}
