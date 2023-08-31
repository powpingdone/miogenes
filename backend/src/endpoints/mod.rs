use crate::{error::MioInnerError, DATA_DIR};
use anyhow::anyhow;
use axum::http::StatusCode;
#[allow(unused)]
use log::*;
use path_absolutize::*;
use std::path::Path;
use uuid::Uuid;

pub mod folders;
pub mod idquery;
pub mod query;
pub mod track_manage;

// util function to check if path is in user path
fn check_dir_in_data_dir(path: impl AsRef<Path>, userid: Uuid) -> Result<(), MioInnerError> {
    debug!("CD_IN_DD checking {:?}", path.as_ref());
    let real_path = DATA_DIR.get().unwrap().join(&format!("{userid}"));
    debug!("CD_IN_DD real path is {real_path:?}");
    let mut ask_path = real_path.clone();
    ask_path.push(&path);
    debug!("CD_IN_DD ask path is {ask_path:?}");
    let real_path = real_path.absolutize()?;
    debug!("CD_IN_DD real canon {real_path:?}");
    let ask_path = match ask_path.absolutize() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(MioInnerError::ExtIoError(
                anyhow!("the path supplied could not be canonicalize'd: {err}"),
                StatusCode::BAD_REQUEST,
            ));
        }
    };
    debug!("CD_IN_DD ask canon {ask_path:?}");
    let real_path = real_path.components().collect::<Vec<_>>();
    let ask_path = ask_path.components().collect::<Vec<_>>();

    // NOTE: this length check is used to make sure that `zip` in the following for
    // loop does not run out and produce an Ok when the ask path is shorter than the
    // real path.
    let ret_err = {
        if cfg!(test) {
            anyhow!("invalid path: {:?}", path.as_ref().as_os_str())
        } else {
            anyhow!("bad path")
        }
    };
    if ask_path.len() < real_path.len() {
        return Err(MioInnerError::ExtIoError(ret_err, StatusCode::BAD_REQUEST));
    }
    for (real, ask) in real_path.iter().zip(ask_path.iter()) {
        if real != ask {
            return Err(MioInnerError::ExtIoError(ret_err, StatusCode::BAD_REQUEST));
        }
    }
    Ok(())
}
