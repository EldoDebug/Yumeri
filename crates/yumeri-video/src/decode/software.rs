use rsmpeg::avcodec::{AVCodecContext, AVCodecParameters};
use rsmpeg::avutil::AVRational;
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use crate::demux::{create_decoder, pts_to_secs};
use crate::error::{Result, VideoError};
use crate::frame::VideoFrame;

use super::{sws_frame_to_cpu, DecoderBackend};

/// Software video decoder: FFmpeg SW decode + sws_scale -> RGBA8 CPU frames.
pub(crate) struct SoftwareDecoder {
    decoder: AVCodecContext,
    sws_ctx: SwsContext,
    width: i32,
    height: i32,
    time_base: AVRational,
}

impl SoftwareDecoder {
    pub fn new(codecpar: &AVCodecParameters, time_base: AVRational) -> Result<Self> {
        let decoder = create_decoder(codecpar, "video")?;
        let width = codecpar.width;
        let height = codecpar.height;
        let src_format = codecpar.format;

        let sws_ctx = SwsContext::get_context(
            width,
            height,
            src_format,
            width,
            height,
            ffi::AV_PIX_FMT_RGBA,
            ffi::SWS_BILINEAR,
            None,
            None,
            None,
        )
        .ok_or_else(|| VideoError::Decode("failed to create sws context".into()))?;

        Ok(Self {
            decoder,
            sws_ctx,
            width,
            height,
            time_base,
        })
    }
}

impl DecoderBackend for SoftwareDecoder {
    fn send_packet(&mut self, packet: &rsmpeg::avcodec::AVPacket) -> Result<()> {
        self.decoder
            .send_packet(Some(packet))
            .map_err(|e| VideoError::Decode(format!("send_packet failed: {e}")))
    }

    fn send_eof(&mut self) -> Result<()> {
        self.decoder
            .send_packet(None)
            .map_err(|e| VideoError::Decode(format!("send_eof failed: {e}")))
    }

    fn decode_next(&mut self) -> Result<Option<VideoFrame>> {
        let frame = match self.decoder.receive_frame() {
            Ok(f) => f,
            Err(RsmpegError::DecoderDrainError) => return Ok(None),
            Err(RsmpegError::DecoderFlushedError) => return Ok(None),
            Err(e) => return Err(VideoError::Decode(format!("receive_frame failed: {e}"))),
        };

        let pts = pts_to_secs(frame.pts, self.time_base);
        let cpu_frame = sws_frame_to_cpu(
            &mut self.sws_ctx,
            &frame,
            self.width as u32,
            self.height as u32,
            pts,
        )?;
        Ok(Some(cpu_frame))
    }

    fn flush(&mut self) {
        unsafe {
            ffi::avcodec_flush_buffers(self.decoder.as_mut_ptr());
        }
    }
}
