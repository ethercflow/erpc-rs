// Copyright (c) 2023, IOMesh Inc. All rights reserved.

use std::{
    error,
    fmt::{self, Debug, Display},
    result,
};

#[derive(Debug)]
pub enum Error {
    /// Codec error.
    Codec(Box<dyn error::Error + Send + Sync>),
    /// Channel error.
    Channel(Box<dyn error::Error + Send + Sync>),
    /// Erpc internal error.
    Internal(String),
}

impl Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Codec(s) => {
                write!(fmt, "codec error: {s:?}")
            }
            Error::Channel(s) => {
                write!(fmt, "channel error: {s:?}")
            }
            Error::Internal(s) => {
                write!(fmt, "internal error: {s:?}")
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Codec(ref e) => Some(e.as_ref()),
            Error::Channel(ref e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<prost::DecodeError> for Error {
    fn from(e: prost::DecodeError) -> Error {
        Error::Codec(Box::new(e))
    }
}

impl From<prost::EncodeError> for Error {
    fn from(e: prost::EncodeError) -> Error {
        Error::Codec(Box::new(e))
    }
}

impl From<async_channel::RecvError> for Error {
    fn from(e: async_channel::RecvError) -> Self {
        Error::Channel(Box::new(e))
    }
}

/// Type alias to use this library's [`Error`] type in a `Result`.
pub type Result<T> = result::Result<T, Error>;
