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
    // get user dir for data 
    let real_path = DATA_DIR.get().unwrap().join(&format!("{userid}"));
    debug!("CD_IN_DD real path is {real_path:?}");
    // then append user requested path
    let mut ask_path = real_path.clone();
    ask_path.push(&path);
    debug!("CD_IN_DD ask path is {ask_path:?}");
    // then resolve both
    let real_path = real_path.absolutize()?;
    let ask_path = match ask_path.absolutize() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(MioInnerError::ExtIoError(
                anyhow!("the path supplied could not be absolutize'd: {err}"),
                StatusCode::BAD_REQUEST,
            ));
        }
    };
    debug!("CD_IN_DD real canon {real_path:?}");
    debug!("CD_IN_DD ask canon {ask_path:?}");
    
    // now, compare paths
    let ret_err = {
        if cfg!(test) {
            anyhow!("invalid path: {:?}", path.as_ref().as_os_str())
        } else {
            anyhow!("bad path")
        }
    };
    let real_path = real_path.components().collect::<Vec<_>>();
    let ask_path = ask_path.components().collect::<Vec<_>>();
    // NOTE: this length check is used to make sure that `zip` in the following for
    // loop does not run out and produce an Ok when the ask path is shorter than the
    // real path.
    if ask_path.len() < real_path.len() {
        return Err(MioInnerError::ExtIoError(ret_err, StatusCode::BAD_REQUEST));
    }
    // if the first components in real match the first components in ask, then it's good.
    // absolutized real and ask will share the same base path, and since they're absolutized,
    // they will not resolve to anywhere else past that
    for (real, ask) in real_path.iter().zip(ask_path.iter()) {
        if real != ask {
            return Err(MioInnerError::ExtIoError(ret_err, StatusCode::BAD_REQUEST));
        }
    }
    Ok(())
}
