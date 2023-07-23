use std::{error::Error, fmt::Display};

pub(crate) type GlueResult<T> = Result<T, ErrorSplit>; 

#[derive(Debug)]
pub enum ErrorSplit {
    Ureq(Box<ureq::Error>),
    Other(anyhow::Error),
}

impl From<ureq::Error> for ErrorSplit {
    fn from(value: ureq::Error) -> Self {
        ErrorSplit::Ureq(Box::new(value))
    }
}

impl From<anyhow::Error> for ErrorSplit {
    fn from(value: anyhow::Error) -> Self {
        ErrorSplit::Other(value)
    }
}

impl From<std::io::Error> for ErrorSplit {
    fn from(value: std::io::Error) -> Self {
        ErrorSplit::Other(value.into())
    }
}

impl Display for ErrorSplit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSplit::Ureq(err) => err.fmt(f),
            ErrorSplit::Other(err) => err.fmt(f),
        }
    }
}

impl Error for ErrorSplit {}
