pub(crate) mod software;
pub(crate) mod vulkan;
pub(crate) mod vulkan_ffi;

use rsmpeg::avcodec::AVCodecParameters;
use rsmpeg::avutil::{AVFrame, AVRational};
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use crate::error::{Result, VideoError};
use crate::frame::VideoFrame;
use crate::pixel_format::VideoPixelFormat;

pub use vulkan::VulkanDeviceInfo;

/// Abstraction over video decode backends.
pub(crate) trait DecoderBackend: Send {
    /// Decode the next frame from a previously sent packet. Returns None when drained.
    fn decode_next(&mut self) -> Result<Option<VideoFrame>>;

    /// Send a packet to the decoder.
    fn send_packet(&mut self, packet: &rsmpeg::avcodec::AVPacket) -> Result<()>;

    /// Send flush signal (EOF) to the decoder.
    fn send_eof(&mut self) -> Result<()>;

    /// Flush decoder buffers (for seek).
    fn flush(&mut self);
}

/// Try to create a Vulkan hwaccel decoder, falling back to software.
pub(crate) fn create_decoder(
    codecpar: &AVCodecParameters,
    time_base: AVRational,
    vulkan_info: Option<&VulkanDeviceInfo>,
) -> Result<Box<dyn DecoderBackend>> {
    if let Some(info) = vulkan_info {
        match vulkan::VulkanDecoder::new(codecpar, time_base, info) {
            Ok(decoder) => return Ok(Box::new(decoder)),
            Err(e) => {
                log::warn!("Vulkan hwaccel unavailable, falling back to software: {e}");
            }
        }
    }

    log::info!("Using software video decoder");
    let decoder = software::SoftwareDecoder::new(codecpar, time_base)?;
    Ok(Box::new(decoder))
}

/// Scale an AVFrame to RGBA8 and copy the pixel data into a `VideoFrame::Cpu`.
/// Shared by both `SoftwareDecoder` and `VulkanDecoder`'s SW fallback path.
pub(crate) fn sws_frame_to_cpu(
    sws: &mut SwsContext,
    frame: &AVFrame,
    width: u32,
    height: u32,
    pts: f64,
) -> Result<VideoFrame> {
    let w = width as i32;
    let h = height as i32;

    let mut dst_frame = AVFrame::new();
    dst_frame.set_format(ffi::AV_PIX_FMT_RGBA);
    dst_frame.set_width(w);
    dst_frame.set_height(h);
    unsafe {
        let ret = ffi::av_frame_get_buffer(dst_frame.as_mut_ptr(), 0);
        if ret < 0 {
            return Err(VideoError::Decode(
                "failed to allocate RGBA frame buffer".into(),
            ));
        }
    }

    sws.scale_frame(frame, 0, h, &mut dst_frame)
        .map_err(|e| VideoError::Decode(format!("sws_scale failed: {e}")))?;

    let stride = unsafe { (*dst_frame.as_ptr()).linesize[0] } as usize;
    let row_bytes = width as usize * 4;
    let src_ptr = unsafe { (*dst_frame.as_ptr()).data[0] };

    let data = if stride == row_bytes {
        unsafe { std::slice::from_raw_parts(src_ptr, row_bytes * height as usize).to_vec() }
    } else {
        let mut data = Vec::with_capacity(row_bytes * height as usize);
        for y in 0..height as usize {
            let row =
                unsafe { std::slice::from_raw_parts(src_ptr.add(y * stride), row_bytes) };
            data.extend_from_slice(row);
        }
        data
    };

    Ok(VideoFrame::Cpu {
        data,
        width,
        height,
        format: VideoPixelFormat::Rgba8,
        pts,
    })
}
