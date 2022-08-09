use std::{num::ParseIntError, sync::PoisonError};

use thiserror::Error as ThisError;

use serde_urlencoded::ser::Error as SerdeUrlEncodedError;
use serde_json::Error as SerdeJsonError;
use serde::de::value::Error as SerdeValueError;
use std::io::Error as IoError;
use std::time::SystemTimeError;

pub type Result<T> = std::result::Result<T, Error>;


#[derive(Debug, ThisError)]
pub enum Error {
    #[error("Poison Error")]
    Poisoned,

    #[error("Serde Json Error: {0}")]
    SerdeJson(#[from] SerdeJsonError),

    #[error("Serde Value Error: {0}")]
    SerdeValue(#[from] SerdeValueError),

    #[error("Serde UrlEncoded Error: {0}")]
    SerdeUrlEncoded(#[from] SerdeUrlEncodedError),

    #[error("IO Error: {0}")]
    Io(#[from] IoError),
    #[error("SystemTime Error: {0}")]
    SystemTime(#[from] SystemTimeError),
    #[error("Parse Int Error: {0}")]
    ParseInt(#[from] ParseIntError),

    #[error("Missing ':' from Source")]
    SourceSplit,
}

impl<V> From<PoisonError<V>> for Error {
    fn from(_: PoisonError<V>) -> Self {
        Self::Poisoned
    }
}