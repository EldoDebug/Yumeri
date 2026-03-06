use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VideoError {
    #[error("failed to read file `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to decode video: {0}")]
    Decode(String),

    #[error("no video track found")]
    NoVideoTrack,

    #[error("playback error: {0}")]
    Playback(String),

    #[error("vulkan hwaccel not available: {0}")]
    VulkanNotAvailable(String),

    #[error("unsupported codec: {0}")]
    UnsupportedCodec(String),
}

pub type Result<T> = std::result::Result<T, VideoError>;
