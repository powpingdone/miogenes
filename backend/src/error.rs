use axum::{
    http::{Response, StatusCode},
    response::IntoResponse,
    Json,
};
use log::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Internal server errors, depending on the issue
#[derive(Debug, Error)]
pub enum MioInnerError {
    #[error("could not find: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    NotFound(anyhow::Error),
    #[error("DATABASE ERROR: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    DbError(anyhow::Error),
    #[error("user challenge failure: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    UserChallengedFail(anyhow::Error, StatusCode),
    #[error("user creation failure: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    UserCreationFail(anyhow::Error, StatusCode),
    #[error("track processing error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    TrackProcessingError(anyhow::Error, StatusCode),
}

// helper function for translating DbErrs to logged statuscodes
impl From<sqlx::Error> for MioInnerError {
    fn from(value: sqlx::Error) -> Self {
        MioInnerError::DbError(anyhow::Error::from(value))
    }
}

impl IntoResponse for MioInnerError {
    fn into_response(self) -> axum::response::Response {
        #[derive(Serialize)]
        struct Error {
            error: String,
        }

        log::log!(
            match self {
                MioInnerError::NotFound(_) => Level::Debug,
                MioInnerError::DbError(_) => Level::Error,
                MioInnerError::UserChallengedFail(_, _) => Level::Info,
                MioInnerError::UserCreationFail(_, _) => Level::Error,
                MioInnerError::TrackProcessingError(_, _) => Level::Error,
            },
            "{}",
            self
        );

        // return
        (
            match self {
                MioInnerError::NotFound(_) => StatusCode::NOT_FOUND,
                MioInnerError::DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
                MioInnerError::UserChallengedFail(_, c)
                | MioInnerError::UserCreationFail(_, c)
                | MioInnerError::TrackProcessingError(_, c) => c,
            },
            Json(Error {
                error: format!(
                    "{}",
                    match self {
                        MioInnerError::NotFound(e)
                        | MioInnerError::UserChallengedFail(e, _)
                        | MioInnerError::UserCreationFail(e, _)
                        | MioInnerError::TrackProcessingError(e, _) => e,
                        MioInnerError::DbError(_) =>
                            anyhow::anyhow!("Internal database error. Please check server log."),
                    }
                ),
            }),
        )
            .into_response()
    }
}
