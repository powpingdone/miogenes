use crate::{error::MioInnerError, DATA_DIR};
use anyhow::anyhow;
use axum::http::StatusCode;
use std::path::Path;
use uuid::Uuid;

pub mod folders;
pub mod idquery;
pub mod query;
pub mod search;
pub mod track_manage;

pub(self) fn check_dir_in_data_dir(
    path: impl AsRef<Path>,
    userid: Uuid,
) -> Result<(), MioInnerError> {
    let real_path = Path::new(&format!("{}/{userid}/", DATA_DIR.get().unwrap())).to_path_buf();
    let mut ask_path = real_path.clone();
    ask_path.push(&path);
    let real_path = real_path.canonicalize()?;
    let ask_path = match ask_path.canonicalize() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(MioInnerError::ExtIoError(
                anyhow!("the path supplied could not be canonicalize'd: {err}"),
                StatusCode::BAD_REQUEST,
            ));
        }
    };
    let real_path = real_path.components().collect::<Vec<_>>();
    let ask_path = ask_path.components().collect::<Vec<_>>();

    // NOTE: this length check is used to make sure that `zip` in the following for
    // loop does not run out and produce an Ok when the ask path is shorter than the
    // real path.
    if ask_path.len() <= real_path.len() {
        return Err(MioInnerError::ExtIoError(
            anyhow!("invalid path: {:?}", path.as_ref().as_os_str()),
            StatusCode::BAD_REQUEST,
        ));
    }
    for (real, ask) in real_path.iter().zip(ask_path.iter()) {
        if real != ask {
            return Err(MioInnerError::ExtIoError(
                anyhow!("invalid path: {:?}", path.as_ref().as_os_str()),
                StatusCode::BAD_REQUEST,
            ));
        }
    }
    Ok(())
}
