use std::{error::Error, fmt::Display};

pub(crate) type GlueResult<T> = Result<T, ErrorSplit>;

#[derive(Debug)]
pub enum ErrorSplit {
    Reqwest(reqwest::Error),
    StdIO(std::io::Error),
    Other(anyhow::Error),
}

impl From<reqwest::Error> for ErrorSplit {
    fn from(value: reqwest::Error) -> Self {
        ErrorSplit::Reqwest(value)
    }
}

impl From<anyhow::Error> for ErrorSplit {
    fn from(value: anyhow::Error) -> Self {
        ErrorSplit::Other(value)
    }
}

impl From<std::io::Error> for ErrorSplit {
    fn from(value: std::io::Error) -> Self {
        ErrorSplit::StdIO(value)
    }
}

impl Display for ErrorSplit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSplit::Reqwest(err) => err.fmt(f),
            ErrorSplit::StdIO(err) => err.fmt(f),
            ErrorSplit::Other(err) => err.fmt(f),
        }
    }
}

impl Error for ErrorSplit {}
