use std::{error::Error, fmt::{Debug, Display}};

pub type GlueResult<T> = Result<T, GlueError>;

pub enum GlueError {
    GSTError(Box<dyn 'static + Display>),
    NoSpaceLeftInQueue,
}

impl From<gstreamer::glib::BoolError> for GlueError {
    fn from(value: gstreamer::glib::BoolError) -> Self {
        GlueError::GSTError(Box::new(value))
    }
}

impl From<gstreamer::StateChangeError> for GlueError {
    fn from(value: gstreamer::StateChangeError) -> Self {
        GlueError::GSTError(Box::new(value))
    }
}

impl Display for GlueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GlueError::GSTError(x) => x.fmt(f),
            GlueError::NoSpaceLeftInQueue => f.write_str("queue is full"),
        }
    }
}

impl Debug for GlueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

impl Error for GlueError {}
