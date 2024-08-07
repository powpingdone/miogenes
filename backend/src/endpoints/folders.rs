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
use mio_protocol::retstructs::FolderQueryItem;
use mio_protocol::*;
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

#[tracing::instrument]
async fn folder_create(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> impl IntoResponse {
    debug!("PUT /api/folder creating folder {name} at {path:?}");
    let name = sanitize_filename::sanitize(name);
    let pbuf = path.into_iter().collect::<PathBuf>();

    // wait for lock to start folder creation, also check dir is where it should be
    let (_hold, task) = tokio::join!(
        state.lock_files.write(),
        tokio::task::spawn_blocking({
            let pbuf = pbuf.clone();
            move || check_dir_in_data_dir(&pbuf, userid)
        })
    );
    task??;
    let pbuf = DATA_DIR
        .get()
        .unwrap()
        .join(format!("{userid}"))
        .join(&pbuf)
        .join(&name);
    create_dir(pbuf).await.map_err(|err| match err.kind() {
        std::io::ErrorKind::AlreadyExists => MioInnerError::Conflict(anyhow!("{name}")),
        _ => MioInnerError::from(err),
    })
}

#[tracing::instrument]
async fn folder_rename(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderRename { old_path, new_path }): Query<msgstructs::FolderRename>,
) -> impl IntoResponse {
    debug!("PATCH /api/folder move from '{old_path:?}' -> '{new_path:?}'");
    let (_hold, check_dir) = tokio::join!(
        state.lock_files.write(),
        // setup vars, majority of blocking code here
        tokio::task::spawn_blocking({
            let old_path = old_path.clone().into_iter().collect::<PathBuf>();
            let new_path = new_path.clone().into_iter().collect::<PathBuf>();
            move || -> Result<_, MioInnerError> {
                check_dir_in_data_dir(&old_path, userid)?;
                check_dir_in_data_dir(&new_path, userid)?;

                // generate paths
                let pbuf: PathBuf = DATA_DIR.get().unwrap().join(format!("{userid}"));
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
        return Err(MioInnerError::NotFound(anyhow!("{old_path:?}")));
    }

    // and make sure it's also a directory
    if !metadata(&old).await?.is_dir() {
        return Err(MioInnerError::ExternalIoError(
            anyhow!("the directory specified is not a directory"),
            StatusCode::BAD_REQUEST,
        ));
    }

    // and that we're not in conflict
    if try_exists(&new).await? {
        return Err(MioInnerError::Conflict(anyhow!("{new_path:?}")));
    }

    // finally, rename
    debug!(
        "PATCH /api/folder finally attempting to move \"{}\" -> \"{}\"",
        old.display(),
        new.display()
    );
    rename(old, new).await.map_err(MioInnerError::from)
}

#[tracing::instrument]
async fn folder_query(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    path_query: Option<Query<msgstructs::FolderQuery>>,
) -> Result<impl IntoResponse, MioInnerError> {
    let _hold = state.lock_files.read().await;
    let top_level_dir = crate::DATA_DIR.get().unwrap().join(format!("{userid}"));
    if let Some(Query(msgstructs::FolderQuery { path })) = path_query {
        let foldname = path.last().map(String::to_owned).unwrap_or_default();
        debug!(
            "GET /api/folder querying folder {}, {path:?}",
            top_level_dir.display()
        );

        // check in dir
        let path = path.into_iter().collect::<PathBuf>();
        let path = path.absolutize()?.to_owned();
        check_dir_in_data_dir(&path, userid)?;
        let mut check = top_level_dir.clone();
        check.push(path);

        // list files of a folder
        let mut ret: Vec<retstructs::FolderQueryItem> = vec![];
        let mut read_dir = read_dir(check).await?;
        while let Some(x) = read_dir.next_entry().await? {
            let loghold = x.file_name();
            let logfile = AsRef::<std::path::Path>::as_ref(&loghold).display();
            trace!("GET /api/folder checking branch {logfile}");
            let ftype = x.file_type().await?;
            if ftype.is_file() {
                trace!("GET /api/folder branch is file {logfile}");

                // check if fname is valid uuid
                Uuid::try_parse(
                    x.file_name().into_string().map_err(|err| {
                        MioInnerError::InternalIoError(
                            anyhow!(
                                "could not convert internal file name into string to become uuid: name is {}",
                                err.to_string_lossy()
                            ),
                        )
                    })?.as_str(),
                ).map_err(
                    |err| MioInnerError::InternalIoError(anyhow!("internal file name is not a uuid: {err}")),
                )?;
                ret.push(retstructs::FolderQueryItem {
                    tree: None,
                    id: x.file_name().into_string().unwrap(),
                    item_type: retstructs::FolderQueryItemType::Audio,
                });
            } else if ftype.is_dir() {
                trace!("GET /api/folder branch is folder {logfile}");
                ret.push(retstructs::FolderQueryItem {
                    tree: None,
                    item_type: retstructs::FolderQueryItemType::Folder,
                    id: x.file_name().into_string().map_err(|osstr| {
                        MioInnerError::InternalIoError(anyhow!(
                            "could not convert internal folder name into string: dir name is {}",
                            osstr.to_string_lossy()
                        ))
                    })?,
                });
            }
        }
        Ok(Json(retstructs::FolderQuery {
            ret: retstructs::FolderQueryItem {
                tree: Some(ret),
                id: foldname,
                item_type: retstructs::FolderQueryItemType::Folder,
            },
        }))
    } else {
        // query folder tree
        debug!("GET /api/folder querying folder tree");
        Ok(Json(retstructs::FolderQuery {
            ret: folder_tree_inner(
                top_level_dir.clone(),
                read_dir(top_level_dir).await?,
                "".into(),
            )
            .await?,
        }))
    }
}

fn folder_tree_inner(
    base_dir: PathBuf,
    mut dir: ReadDir,
    curr_path: PathBuf,
) -> Pin<Box<dyn Future<Output = Result<FolderQueryItem, MioInnerError>> + Send>> {
    // The strange return type is due to recursion. I don't think that this could have
    // been impl'd reasonably any other way. Thanks rust async!
    Box::pin(async move {
        debug!("FTI call to ({base_dir:?}, {curr_path:?})");

        fn fail_to_utf8(file: impl AsRef<std::path::Path>) -> MioInnerError {
            MioInnerError::InternalIoError(anyhow!(
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

        // 2: for each dir, recurse
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

        // 3: collect, and then add to self
        let mut dirs_finished = vec![];
        for fut in dir_futs {
            dirs_finished.push(fut.await??);
        }
        Ok(retstructs::FolderQueryItem {
            // return none if there was no children
            tree: if dirs_finished.is_empty() {
                None
            } else {
                Some(dirs_finished)
            },
            item_type: retstructs::FolderQueryItemType::Folder,
            id: curr_path
                .file_name()
                .unwrap_or_default()
                .to_owned()
                .into_string()
                .map_err(fail_to_utf8)?,
        })
    })
}

#[tracing::instrument]
async fn folder_delete(
    State(state): State<MioState>,
    Extension(auth::JWTInner { userid, .. }): Extension<auth::JWTInner>,
    Query(msgstructs::FolderCreateDelete { name, path }): Query<msgstructs::FolderCreateDelete>,
) -> Result<impl IntoResponse, MioInnerError> {
    let inp_path = path.into_iter().collect::<PathBuf>();
    let real_path = crate::DATA_DIR
        .get()
        .unwrap()
        .join(format!("{userid}"))
        .join(&inp_path)
        .join(&name);
    debug!(
        "DELETE /api/folder attemping to delete folder {}",
        real_path.display()
    );
    let (_hold, ret) = tokio::join!(
        state.lock_files.write(),
        tokio::task::spawn_blocking({
            let path = inp_path.clone();
            move || check_dir_in_data_dir(&path, userid)
        })
    );
    ret??;

    // check if dir has contents https://github.com/rust-lang/rust/issues/86442
    if read_dir(&real_path).await?.next_entry().await?.is_some() {
        return Err(MioInnerError::ExternalIoError(
            anyhow!(
                "Directory {:?} has items, please remove them.",
                inp_path
                    .clone()
                    .into_iter()
                    .chain(std::iter::once(name.as_ref()))
                    .collect::<PathBuf>()
                    .display()
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
    use crate::test::*;
    use axum::http::{Method, StatusCode};
    use mio_protocol::*;
    use serde_urlencoded::to_string as url_enc;
    use std::collections::HashSet;

    // util function to check if dirs are the same
    #[must_use]
    async fn tree_check(cli: &axum_test::TestServer, jwt: &auth::JWT, dirs: &[&str]) -> bool {
        let ret = jwt_header(cli, Method::GET, "/api/folder", jwt)
            .await
            .json::<retstructs::FolderQuery>()
            .ret;
        let mut dirs_reconstruct: retstructs::FolderQueryItem = retstructs::FolderQueryItem {
            tree: None,
            id: "".to_owned(),
            item_type: retstructs::FolderQueryItemType::Folder,
        };
        for dir in dirs {
            push_recurse(&mut dirs_reconstruct, dir)
        }
        dbg!(ret) == dbg!(dirs_reconstruct)
    }

    // util function to reconstruct dirs
    fn push_recurse(master: &mut retstructs::FolderQueryItem, path_slice: &str) {
        if path_slice.is_empty() {
            return;
        }
        let next_pt = path_slice.find('/');
        if let Some(pt) = next_pt {
            // not at end
            let (this_dir, remaining_dirs) = path_slice.split_at(pt);
            let remaining_dirs = remaining_dirs.trim_start_matches("/");
            if master.tree.is_none() {
                master.tree = Some(vec![]);
            }
            let tree = master.tree.as_mut().unwrap();

            // find item
            for i in tree.iter_mut() {
                if i.id == this_dir {
                    push_recurse(i, remaining_dirs);
                    return;
                }
            }

            // item not found
            let mut new = retstructs::FolderQueryItem {
                tree: None,
                id: this_dir.to_owned(),
                item_type: retstructs::FolderQueryItemType::Folder,
            };
            push_recurse(&mut new, remaining_dirs);
            tree.push(new);
        } else {
            // at end
            if master.tree.is_none() {
                master.tree = Some(vec![retstructs::FolderQueryItem {
                    tree: None,
                    id: path_slice.to_owned(),
                    item_type: retstructs::FolderQueryItemType::Folder,
                }]);
            } else {
                // check for collisions
                for i in master.tree.as_ref().unwrap().iter() {
                    if i.id == path_slice {
                        return;
                    }
                }
                master
                    .tree
                    .as_mut()
                    .unwrap()
                    .push(retstructs::FolderQueryItem {
                        tree: None,
                        id: path_slice.to_owned(),
                        item_type: retstructs::FolderQueryItemType::Folder,
                    })
            }
        }
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
                    path: vec!["".to_string()],
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
                    path: vec!["a horse".to_string(), "".to_string()],
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
                    path: vec!["a horse".to_string(), "neigh".to_string()],
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
                &["a horse", "a horse/neigh", "a horse/neigh/bleh"]
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
                    old_path: vec!["a horse".to_string()],
                    new_path: vec!["merasmus".to_string()],
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
                &["merasmus", "merasmus/neigh", "merasmus/neigh/bleh"]
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
                    old_path: vec![
                        "merasmus".to_string(),
                        "neigh".to_string(),
                        "bleh".to_string()
                    ],
                    new_path: vec!["merasmus".to_string(), "bleh".to_string()],
                })
                .unwrap()
            ),
            &jwt,
        )
        .await;
        assert!(tree_check(&cli, &jwt, &["merasmus", "merasmus/neigh", "merasmus/bleh"]).await);

        // delete
        jwt_header(
            &cli,
            Method::DELETE,
            "/api/folder?name=bleh&path=merasmus",
            &jwt,
        )
        .await;
        assert!(tree_check(&cli, &jwt, &["merasmus", "merasmus/neigh"]).await);
    }

    #[tokio::test]
    #[ignore]
    // this test _MAY MODIFY FILES ON YOUR HARD DRIVE NOT IN THE TEST DIR_, please
    // only run as a verification of your changes
    async fn folder_bad_path_checks() {
        let cli = client().await;
        let jwt = gen_user(&cli, "folder_bad_path_checks").await;
        #[allow(non_snake_case)]
        let TEST_PATHS: Vec<Vec<String>> = vec![
            vec![".."],
            vec!["..", ""],
            vec!["a", "..", ".."],
            vec!["..", "..", "test_files"],
        ]
        .into_iter()
        .map(|x| x.into_iter().map(str::to_owned).collect())
        .collect();
        let err = retstructs::ErrorMsg {
            error: crate::MioInnerError::ExternalIoError(
                anyhow::anyhow!("bad path"),
                StatusCode::BAD_REQUEST,
            )
            .msg(),
        };
        for path in TEST_PATHS {
            for name in path.iter() {
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::PUT,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderCreateDelete {
                                name: name.clone(),
                                path: path.clone(),
                            })
                            .unwrap()
                        ),
                        &jwt
                    )
                    .expect_failure()
                    .await
                    .json::<retstructs::ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::GET,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderQuery { path: path.clone() }).unwrap()
                        ),
                        &jwt,
                    )
                    .expect_failure()
                    .await
                    .json::<retstructs::ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::PATCH,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderRename {
                                old_path: path.clone(),
                                new_path: path.clone(),
                            })
                            .unwrap()
                        ),
                        &jwt
                    )
                    .expect_failure()
                    .await
                    .json::<retstructs::ErrorMsg>(),
                    err
                );
                assert_eq!(
                    jwt_header(
                        &cli,
                        Method::DELETE,
                        &format!(
                            "/api/folder?{}",
                            url_enc(msgstructs::FolderCreateDelete {
                                name: name.clone(),
                                path: path.clone(),
                            })
                            .unwrap()
                        ),
                        &jwt,
                    )
                    .expect_failure()
                    .await
                    .json::<retstructs::ErrorMsg>(),
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
            assert!(tree_check(&cli, &jwt, &["a", "a/b", "1"]).await)
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
