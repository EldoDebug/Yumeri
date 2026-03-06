use std::ffi::CString;
use std::path::Path;

use rsmpeg::ffi;

use crate::demux::Demuxer;
use crate::error::{Result, VideoError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VideoCodec {
    H264,
    H265,
    Av1,
    Vp9,
    Other(u32),
}

/// Lightweight metadata container for a video file (no decoding, probe only).
pub struct Video {
    width: u32,
    height: u32,
    frame_rate: f64,
    duration_secs: f64,
    codec: VideoCodec,
    has_audio: bool,
}

impl Video {
    /// Probe a video file for metadata without decoding any frames.
    pub fn probe(path: impl AsRef<Path>) -> Result<Self> {
        let demuxer = Demuxer::open(path.as_ref())?;

        let codec = match demuxer.video_codecpar().codec_id {
            ffi::AV_CODEC_ID_H264 => VideoCodec::H264,
            ffi::AV_CODEC_ID_HEVC => VideoCodec::H265,
            ffi::AV_CODEC_ID_AV1 => VideoCodec::Av1,
            ffi::AV_CODEC_ID_VP9 => VideoCodec::Vp9,
            other => VideoCodec::Other(other as u32),
        };

        Ok(Self {
            width: demuxer.video_width(),
            height: demuxer.video_height(),
            frame_rate: demuxer.video_frame_rate(),
            duration_secs: demuxer.duration_secs(),
            codec,
            has_audio: demuxer.has_audio(),
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn frame_rate(&self) -> f64 {
        self.frame_rate
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    pub fn codec(&self) -> VideoCodec {
        self.codec
    }

    pub fn has_audio(&self) -> bool {
        self.has_audio
    }
}

pub(crate) fn path_to_cstring(path: &Path) -> Result<CString> {
    let path_str = path.to_string_lossy();
    CString::new(path_str.as_bytes()).map_err(|e| VideoError::Io {
        path: path.to_path_buf(),
        source: std::io::Error::new(std::io::ErrorKind::InvalidInput, e),
    })
}

pub(crate) fn stream_frame_rate(stream: &rsmpeg::avformat::AVStreamRef<'_>) -> f64 {
    let r = stream.r_frame_rate;
    if r.den > 0 {
        r.num as f64 / r.den as f64
    } else {
        let avg = stream.avg_frame_rate;
        if avg.den > 0 {
            avg.num as f64 / avg.den as f64
        } else {
            30.0
        }
    }
}
