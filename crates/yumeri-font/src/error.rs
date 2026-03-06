use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FontError {
    #[error("failed to read font file `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("invalid font data: no font faces found")]
    InvalidFontData,
}

pub type Result<T> = std::result::Result<T, FontError>;
