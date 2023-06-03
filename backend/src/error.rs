use axum::{
    http::{
        Response,
        StatusCode,
    },
    response::IntoResponse,
    Json,
};
use log::*;
use serde::{
    Deserialize,
    Serialize,
};
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
    #[error("external io error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    ExtIoError(anyhow::Error, StatusCode),
    #[error("internal io error: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    IntIoError(anyhow::Error),
    #[error("conflict found: `{0}`. \nTRACE:\n{}", .0.backtrace())]
    Conflict(anyhow::Error),
}

// helper function for translating DbErrs to logged statuscodes
impl From<sqlx::Error> for MioInnerError {
    fn from(value: sqlx::Error) -> Self {
        MioInnerError::DbError(anyhow::Error::from(value))
    }
}

// internal std::io::error translator
impl From<std::io::Error> for MioInnerError {
    fn from(value: std::io::Error) -> Self {
        MioInnerError::IntIoError(anyhow::Error::from(value))
    }
}

impl IntoResponse for MioInnerError {
    fn into_response(self) -> axum::response::Response {
        use MioInnerError::*;

        #[derive(Serialize)]
        struct Error {
            error: String,
        }

        log::log!(match self {
            NotFound(_) | Conflict(_) | ExtIoError(_, _) => Level::Debug,
            UserChallengedFail(_, _) => Level::Info,
            TrackProcessingError(_, _) => Level::Warn,
            DbError(_) | UserCreationFail(_, _) | IntIoError(_) => Level::Error,
        }, "{}", self);

        // return
        (match self {
            NotFound(_) => StatusCode::NOT_FOUND,
            Conflict(_) => StatusCode::CONFLICT,
            IntIoError(_) | DbError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            UserChallengedFail(_, c) | UserCreationFail(_, c) | TrackProcessingError(_, c) | ExtIoError(_, c) => c,
        }, Json(Error { error: format!("{}", match self {
            NotFound(e) |
            Conflict(e) |
            UserChallengedFail(e, _) |
            UserCreationFail(e, _) |
            TrackProcessingError(e, _) |
            IntIoError(e) |
            ExtIoError(e, _) => e,
            DbError(_) => anyhow::anyhow!("Internal database error. Please check server log."),
        }) })).into_response()
    }
}
