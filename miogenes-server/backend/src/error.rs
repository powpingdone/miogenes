use axum::http::StatusCode;
use log::*;
use mio_migration::DbErr;
use sea_orm::TransactionError;
use thiserror::Error;

// Internal server errors, depending on the issue
#[derive(Debug, Error)]
pub enum MioInnerError {
    #[error("could not find: `{1}`")]
    NotFound(Level, anyhow::Error),
    #[error("DATABASE ERROR: `{1}`")]
    DbError(Level, anyhow::Error),
    #[error("user challenge failure: `{1}`")]
    UserChallengedFail(Level, anyhow::Error, StatusCode),
    #[error("user creation failure: `{1}`")]
    UserCreationFail(Level, anyhow::Error, StatusCode),
    #[error("track processing error: `{1}`")]
    TrackProcessingError(Level, anyhow::Error, StatusCode),
}

// DbErr -> MioInnerError, mostly for logging and StatusCode purposes
impl From<sea_orm::DbErr> for MioInnerError {
    fn from(err: sea_orm::DbErr) -> Self {
        MioInnerError::DbError(Level::Error, anyhow::Error::new(err))
    }
}

// actual logic that does the StatusCode and logging 
#[allow(clippy::from_over_into)]
impl Into<StatusCode> for MioInnerError {
    fn into(self) -> StatusCode {
        // log errors
        log!(match self {
            MioInnerError::NotFound(lvl, _) |
            MioInnerError::DbError(lvl, _) |
            MioInnerError::UserChallengedFail(lvl, _, _) |
            MioInnerError::UserCreationFail(lvl, _, _) |
            MioInnerError::TrackProcessingError(lvl, _, _) => lvl,
        }, "{self}");

        // return status
        match self {
            MioInnerError::NotFound(..) => StatusCode::NOT_FOUND,
            MioInnerError::DbError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            MioInnerError::UserChallengedFail(_, _, code) |
            MioInnerError::UserCreationFail(_, _, code) |
            MioInnerError::TrackProcessingError(_, _, code) => code,
        }
    }
}

// helper function for translating DbErrs to logged statuscodes
pub fn db_err(err: DbErr) -> StatusCode {
    MioInnerError::from(err).into()
}

// change a transaction error to a StatusCode
pub fn tr_conv_code(err: TransactionError<MioInnerError>) -> StatusCode {
    match err {
        TransactionError::Connection(err) => db_err(err),
        TransactionError::Transaction(err) => err.into(),
    }
}
