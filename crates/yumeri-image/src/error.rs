use std::path::PathBuf;

use thiserror::Error;

use crate::PixelFormat;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImageError {
    #[error("failed to read file `{path}`: {source}")]
    Io { path: PathBuf, source: std::io::Error },

    #[error("failed to decode image: {0}")]
    Decode(String),

    #[error("unsupported pixel format conversion: {from:?} -> {to:?}")]
    UnsupportedConversion { from: PixelFormat, to: PixelFormat },
}

pub type Result<T> = std::result::Result<T, ImageError>;
