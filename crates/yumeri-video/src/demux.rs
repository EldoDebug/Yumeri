use std::path::Path;

use rsmpeg::avcodec::{AVCodecContext, AVCodecParameters};
use rsmpeg::avformat::AVFormatContextInput;
use rsmpeg::avutil::AVRational;
use rsmpeg::ffi;

use crate::error::{Result, VideoError};
use crate::video::{path_to_cstring, stream_frame_rate};

pub(crate) enum DemuxPacket {
    Video(rsmpeg::avcodec::AVPacket),
    Audio(rsmpeg::avcodec::AVPacket),
    Eof,
}

/// Demuxer: opens a media file and separates video/audio packet streams.
pub(crate) struct Demuxer {
    input_ctx: AVFormatContextInput,
    video_stream_index: i32,
    audio_stream_index: i32,
    video_time_base: AVRational,
    video_codecpar: AVCodecParameters,
    audio_codecpar: Option<AVCodecParameters>,
    #[allow(dead_code)] // Reserved for future A/V sync improvements
    audio_time_base: AVRational,
    duration_secs: f64,
    video_width: u32,
    video_height: u32,
    video_frame_rate: f64,
}

impl Demuxer {
    pub fn open(path: &Path) -> Result<Self> {
        let c_path = path_to_cstring(path)?;

        let input_ctx = AVFormatContextInput::open(&c_path).map_err(|e| VideoError::Io {
            path: path.to_path_buf(),
            source: std::io::Error::other(e.to_string()),
        })?;

        let (video_idx, _) = input_ctx
            .find_best_stream(ffi::AVMEDIA_TYPE_VIDEO)
            .map_err(|e| VideoError::Decode(e.to_string()))?
            .ok_or(VideoError::NoVideoTrack)?;

        let audio_result = input_ctx
            .find_best_stream(ffi::AVMEDIA_TYPE_AUDIO)
            .ok()
            .flatten();

        let streams = input_ctx.streams();

        let video_time_base = streams[video_idx].time_base;
        let video_codecpar = streams[video_idx].codecpar().clone();
        let video_width = video_codecpar.width as u32;
        let video_height = video_codecpar.height as u32;
        let video_frame_rate = stream_frame_rate(&streams[video_idx]);

        let (audio_stream_index, audio_time_base, audio_codecpar) = match audio_result {
            Some((idx, _)) => (
                idx as i32,
                streams[idx].time_base,
                Some(streams[idx].codecpar().clone()),
            ),
            None => (-1, AVRational { num: 0, den: 1 }, None),
        };

        let duration_secs = if input_ctx.duration > 0 {
            input_ctx.duration as f64 / ffi::AV_TIME_BASE as f64
        } else {
            0.0
        };

        Ok(Self {
            input_ctx,
            video_stream_index: video_idx as i32,
            audio_stream_index,
            video_time_base,
            video_codecpar,
            audio_codecpar,
            audio_time_base,
            duration_secs,
            video_width,
            video_height,
            video_frame_rate,
        })
    }

    pub fn read_packet(&mut self) -> Result<DemuxPacket> {
        loop {
            match self.input_ctx.read_packet() {
                Ok(Some(pkt)) => {
                    let stream_index = pkt.stream_index;
                    if stream_index == self.video_stream_index {
                        return Ok(DemuxPacket::Video(pkt));
                    } else if stream_index == self.audio_stream_index {
                        return Ok(DemuxPacket::Audio(pkt));
                    }
                }
                Ok(None) => return Ok(DemuxPacket::Eof),
                Err(e) => return Err(VideoError::Decode(e.to_string())),
            }
        }
    }

    pub fn seek(&mut self, timestamp_us: i64) -> Result<()> {
        let timestamp = if self.video_time_base.den > 0 {
            // Use i128 to avoid overflow with large timestamps * time_base denominators
            (timestamp_us as i128 * self.video_time_base.den as i128
                / (self.video_time_base.num as i128 * 1_000_000)) as i64
        } else {
            timestamp_us
        };

        self.input_ctx
            .seek(
                self.video_stream_index,
                timestamp,
                ffi::AVSEEK_FLAG_BACKWARD as i32,
            )
            .map_err(|e| VideoError::Decode(format!("seek failed: {e}")))?;

        Ok(())
    }

    pub fn has_audio(&self) -> bool {
        self.audio_stream_index >= 0
    }

    pub fn video_time_base(&self) -> AVRational {
        self.video_time_base
    }

    #[allow(dead_code)] // Reserved for future A/V sync improvements
    pub fn audio_time_base(&self) -> AVRational {
        self.audio_time_base
    }

    pub fn video_codecpar(&self) -> &AVCodecParameters {
        &self.video_codecpar
    }

    pub fn audio_codecpar(&self) -> Option<&AVCodecParameters> {
        self.audio_codecpar.as_ref()
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration_secs
    }

    pub fn video_width(&self) -> u32 {
        self.video_width
    }

    pub fn video_height(&self) -> u32 {
        self.video_height
    }

    pub fn video_frame_rate(&self) -> f64 {
        self.video_frame_rate
    }
}

pub(crate) fn create_decoder(codecpar: &AVCodecParameters, label: &str) -> Result<AVCodecContext> {
    let codec_id = codecpar.codec_id;
    let codec = rsmpeg::avcodec::AVCodec::find_decoder(codec_id)
        .ok_or_else(|| VideoError::UnsupportedCodec(format!("{label} codec id {codec_id}")))?;

    let mut decoder_ctx = AVCodecContext::new(&codec);
    let ret = unsafe {
        ffi::avcodec_parameters_to_context(decoder_ctx.as_mut_ptr(), codecpar.as_ptr())
    };
    if ret < 0 {
        return Err(VideoError::Decode(format!(
            "avcodec_parameters_to_context failed: {ret}"
        )));
    }

    // Enable FFmpeg multithreaded decoding (0 = auto-detect thread count)
    unsafe {
        (*decoder_ctx.as_mut_ptr()).thread_count = 0;
    }

    decoder_ctx
        .open(None)
        .map_err(|e| VideoError::Decode(format!("failed to open {label} decoder: {e}")))?;

    Ok(decoder_ctx)
}

/// Convert pts from stream time_base to seconds.
pub(crate) fn pts_to_secs(pts: i64, time_base: AVRational) -> f64 {
    if pts == ffi::AV_NOPTS_VALUE || time_base.den == 0 {
        0.0
    } else {
        pts as f64 * time_base.num as f64 / time_base.den as f64
    }
}
