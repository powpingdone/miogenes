use anyhow::anyhow;
use axum::{http::StatusCode, response::IntoResponse, Json};
use log::*;
use mio_protocol::retstructs;
use thiserror::Error;
use MioInnerError::*;

// Internal server errors, depending on the issue
#[derive(Debug, Error)]
pub enum MioInnerError {
    #[error("could not find: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    NotFound(anyhow::Error),
    #[error("DATABASE ERROR: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    DbError(anyhow::Error),
    #[error("internal thread issue: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    Panicked(anyhow::Error),
    #[error("user challenge failure: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    UserChallengedFail(anyhow::Error, StatusCode),
    #[error("user creation failure: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    UserCreationFail(anyhow::Error, StatusCode),
    #[error("track processing error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    TrackProcessingError(anyhow::Error, StatusCode),
    #[error("external io error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    ExtIoError(anyhow::Error, StatusCode),
    #[error("internal io error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    IntIoError(anyhow::Error),
    #[error("conflict found: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    Conflict(anyhow::Error),
}

impl MioInnerError {
    // this is mainly outside and pub because of testing
    pub fn msg(self) -> String {
        format!(
            "{}",
            match self {
                NotFound(e)
                | Conflict(e)
                | UserChallengedFail(e, _)
                | UserCreationFail(e, _)
                | TrackProcessingError(e, _)
                | ExtIoError(e, _) => e,
                // these errors are not put into the error field as they could leak information
                IntIoError(_) => {
                    anyhow!("The server encountered an filesystem io error. Please check the server log.")
                }
                Panicked(_) => {
                    anyhow!("The server encountered an error it could not handle. Please check the server log.")
                }
                DbError(_) => {
                    anyhow!("The server encountered an internal database error. Please check server log.")
                }
            }
        )
    }
}

// various helper functions for translating common errors
impl From<tokio::task::JoinError> for MioInnerError {
    fn from(value: tokio::task::JoinError) -> Self {
        Panicked(anyhow::Error::from(value))
    }
}

impl From<sqlx::Error> for MioInnerError {
    fn from(value: sqlx::Error) -> Self {
        DbError(anyhow::Error::from(value))
    }
}

impl From<std::io::Error> for MioInnerError {
    fn from(value: std::io::Error) -> Self {
        IntIoError(anyhow::Error::from(value))
    }
}

impl IntoResponse for MioInnerError {
    fn into_response(self) -> axum::response::Response {
        log::log!(
            match self {
                NotFound(_) | Conflict(_) | ExtIoError(_, _) => Level::Debug,
                UserChallengedFail(_, _) => Level::Info,
                TrackProcessingError(_, _) => Level::Warn,
                DbError(_) | UserCreationFail(_, _) | IntIoError(_) | Panicked(_) => Level::Error,
            },
            "{}",
            self
        );

        // return
        (
            match self {
                NotFound(_) => StatusCode::NOT_FOUND,
                Conflict(_) => StatusCode::CONFLICT,
                IntIoError(_) | DbError(_) | Panicked(_) => StatusCode::INTERNAL_SERVER_ERROR,
                UserChallengedFail(_, c)
                | UserCreationFail(_, c)
                | TrackProcessingError(_, c)
                | ExtIoError(_, c) => c,
            },
            Json(retstructs::ErrorMsg { error: self.msg() }),
        )
            .into_response()
    }
}
