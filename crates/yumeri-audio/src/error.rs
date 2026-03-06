use std::path::PathBuf;

use thiserror::Error;

use crate::SampleFormat;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AudioError {
    #[error("failed to read file `{path}`: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to decode audio: {0}")]
    Decode(String),

    #[error("no audio track found")]
    NoAudioTrack,

    #[error("playback error: {0}")]
    Playback(String),

    #[error("no audio output device available")]
    NoOutputDevice,

    #[error("unsupported sample format conversion: {from:?} -> {to:?}")]
    UnsupportedConversion {
        from: SampleFormat,
        to: SampleFormat,
    },
}

pub type Result<T> = std::result::Result<T, AudioError>;
