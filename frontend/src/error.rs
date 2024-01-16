use std::fmt::Display;

pub type MFResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // Slightly deeper error
    Glue(mio_glue::error::ErrorSplit),
    // Could not upgrade
    StrongGoneRuntime,
    StrongGoneApp,
    StrongGoneState,
    StrongGonePlayer,
    // babe what if slint exploded
    SlintPlatformError(slint::PlatformError),
    // babe I don't think that the gui exists anyway
    //
    // YES DO AS I SAY- *panics*
    //
    // this seriously needs some fuckin unbraindamage
    SlintEventLoopError(slint::EventLoopError),
    // The user did something dumb, and we caught it clientside
    ClientSide(String),
}

impl From<slint::EventLoopError> for Error {
    fn from(value: slint::EventLoopError) -> Self {
        Error::SlintEventLoopError(value)
    }
}

impl From<slint::PlatformError> for Error {
    fn from(value: slint::PlatformError) -> Self {
        Error::SlintPlatformError(value)
    }
}

impl From<mio_glue::error::ErrorSplit> for Error {
    fn from(value: mio_glue::error::ErrorSplit) -> Self {
        Error::Glue(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Glue(err) => err.fmt(f),
            Error::SlintPlatformError(err) => err.fmt(f),
            Error::StrongGoneRuntime => f.write_str("runtime could not be upgraded to strong"),
            Error::StrongGoneApp => f.write_str("slint app could not be upgraded to strong"),
            Error::StrongGoneState => f.write_str("state could not be upgraded to strong"),
            Error::StrongGonePlayer => f.write_str("player could not be upgraded to strong"),
            Error::SlintEventLoopError(err) => err.fmt(f),
            Error::ClientSide(err) => f.write_str(&err),
        }
    }
}

impl std::error::Error for Error {}
