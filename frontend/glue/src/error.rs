use std::{error::Error, fmt::Display};

pub(crate) type GlueResult<T> = Result<T, GlueError>;

#[derive(Debug)]
pub enum GlueError {
    Reqwest(reqwest::Error),
    StdIO(std::io::Error),
    Other(anyhow::Error),
}

impl From<reqwest::Error> for GlueError {
    fn from(value: reqwest::Error) -> Self {
        GlueError::Reqwest(value)
    }
}

impl From<anyhow::Error> for GlueError {
    fn from(value: anyhow::Error) -> Self {
        GlueError::Other(value)
    }
}

impl From<std::io::Error> for GlueError {
    fn from(value: std::io::Error) -> Self {
        GlueError::StdIO(value)
    }
}

impl Display for GlueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlueError::Reqwest(err) => err.fmt(f),
            GlueError::StdIO(err) => err.fmt(f),
            GlueError::Other(err) => err.fmt(f),
        }
    }
}

impl Error for GlueError {}
