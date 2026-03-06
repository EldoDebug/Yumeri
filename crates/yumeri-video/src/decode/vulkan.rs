use std::ffi::{c_char, CString};
use std::sync::Arc;

use ash::vk;
use rsmpeg::avcodec::{AVCodecContext, AVCodecParameters};
use rsmpeg::avutil::{AVFrame, AVHWDeviceContext, AVRational};
use rsmpeg::error::RsmpegError;
use rsmpeg::ffi;
use rsmpeg::swscale::SwsContext;

use crate::demux::pts_to_secs;
use crate::error::{Result, VideoError};
use crate::frame::{FrameRef, GpuFrame, VideoFrame};

use super::vulkan_ffi::{AVVkFrame, AVVulkanDeviceContext, AVVulkanDeviceQueueFamily};
use super::DecoderBackend;

/// Vulkan device handles needed by FFmpeg for hardware-accelerated decoding.
///
/// Created by the renderer and passed to `VideoPlayer` so the decode thread can
/// share the same VkDevice.
#[derive(Clone)]
pub struct VulkanDeviceInfo {
    pub get_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    pub instance: vk::Instance,
    pub physical_device: vk::PhysicalDevice,
    pub device: vk::Device,
    pub graphics_queue_family: u32,
    pub video_decode_queue_family: Option<u32>,
    pub enabled_device_extensions: Vec<CString>,
}

// Safety: raw Vulkan handles are just pointer-sized integers, safe to send across threads.
unsafe impl Send for VulkanDeviceInfo {}
unsafe impl Sync for VulkanDeviceInfo {}

/// Video decoder using FFmpeg's Vulkan hwaccel. Decoded frames stay on the GPU
/// as VkImages (NV12), avoiding CPU-side decode entirely.
pub(crate) struct VulkanDecoder {
    decoder: AVCodecContext,
    // Prevent early drop — FFmpeg references the device context throughout decoding.
    _hw_device_ctx: AVHWDeviceContext,
    // Keep extension C-strings and pointer array alive for FFmpeg.
    _ext_storage: Vec<CString>,
    _ext_ptrs: Vec<*const c_char>,
    time_base: AVRational,
    width: u32,
    height: u32,
    // ash::Device for calling vkWaitSemaphores on decoded frames.
    // Does NOT own the VkDevice — ash::Device has no Drop impl.
    ash_device: ash::Device,
    // Lazy SW fallback: created when FFmpeg produces non-Vulkan frames
    sw_fallback: Option<SwsContext>,
    sw_warned: bool,
}

// Safety: All fields are only accessed from the decode thread after construction.
// _ext_ptrs contains self-referential pointers into _ext_storage.
// AVCodecContext and AVHWDeviceContext wrap FFmpeg C objects that are not
// inherently thread-safe, but are confined to a single thread.
unsafe impl Send for VulkanDecoder {}

impl VulkanDecoder {
    pub fn new(
        codecpar: &AVCodecParameters,
        time_base: AVRational,
        info: &VulkanDeviceInfo,
    ) -> Result<Self> {
        let width = codecpar.width as u32;
        let height = codecpar.height as u32;

        // Keep extension strings alive
        let ext_storage = info.enabled_device_extensions.clone();
        let ext_ptrs: Vec<*const c_char> = ext_storage.iter().map(|s| s.as_ptr()).collect();

        let ext_names: Vec<_> = ext_storage.iter().map(|s| s.to_string_lossy()).collect();
        log::info!(
            "Vulkan hwaccel: trying codec_id={}, {}x{}, \
             video_decode_queue={:?}, extensions=[{}]",
            codecpar.codec_id, width, height,
            info.video_decode_queue_family,
            ext_names.join(", "),
        );

        let hw_device_ctx = Self::create_hw_device_ctx(info, &ext_ptrs)?;

        let decoder = Self::create_decoder(codecpar, &hw_device_ctx)?;

        // Load ash::Device for calling vkWaitSemaphores on decoded frames.
        let get_proc_addr = info.get_proc_addr;
        let instance_fn = ash::InstanceFnV1_0::load(|name| unsafe {
            std::mem::transmute(get_proc_addr(info.instance, name.as_ptr()))
        });
        let ash_device = unsafe { ash::Device::load(&instance_fn, info.device) };

        log::info!("Vulkan hardware decoder initialized ({width}x{height})");

        Ok(Self {
            decoder,
            _hw_device_ctx: hw_device_ctx,
            _ext_storage: ext_storage,
            _ext_ptrs: ext_ptrs,
            time_base,
            width,
            height,
            ash_device,
            sw_fallback: None,
            sw_warned: false,
        })
    }

    fn create_hw_device_ctx(
        info: &VulkanDeviceInfo,
        ext_ptrs: &[*const c_char],
    ) -> Result<AVHWDeviceContext> {
        let mut hw_device_ctx = AVHWDeviceContext::alloc(ffi::AV_HWDEVICE_TYPE_VULKAN);

        // Access the type-specific hwctx
        let ffi_hw_ctx = hw_device_ctx.data as *mut ffi::AVHWDeviceContext;
        let vulkan_ctx = unsafe { (*ffi_hw_ctx).hwctx as *mut AVVulkanDeviceContext };

        unsafe {
            let ctx = &mut *vulkan_ctx;
            ctx.get_proc_addr = info.get_proc_addr;
            ctx.inst = info.instance;
            ctx.phys_dev = info.physical_device;
            ctx.act_dev = info.device;

            // Device extensions
            ctx.enabled_dev_extensions = ext_ptrs.as_ptr();
            ctx.nb_enabled_dev_extensions = ext_ptrs.len() as i32;

            // Deprecated queue fields (required while FF_API_VULKAN_FIXED_QUEUES is active)
            let gfx = info.graphics_queue_family as i32;
            ctx.queue_family_index = gfx;
            ctx.nb_graphics_queues = 1;
            ctx.queue_family_tx_index = gfx;
            ctx.nb_tx_queues = 1;
            ctx.queue_family_comp_index = gfx;
            ctx.nb_comp_queues = 1;
            ctx.queue_family_encode_index = -1;
            ctx.nb_encode_queues = 0;
            ctx.queue_family_decode_index = info
                .video_decode_queue_family
                .map(|f| f as i32)
                .unwrap_or(-1);
            ctx.nb_decode_queues =
                if info.video_decode_queue_family.is_some() { 1 } else { 0 };

            // New-style queue families (qf array)
            let mut nb_qf = 0i32;

            ctx.qf[nb_qf as usize] = AVVulkanDeviceQueueFamily {
                idx: gfx,
                num: 1,
                flags: vk::QueueFlags::GRAPHICS.as_raw()
                    | vk::QueueFlags::COMPUTE.as_raw()
                    | vk::QueueFlags::TRANSFER.as_raw(),
                video_caps: 0,
            };
            nb_qf += 1;

            if let Some(vd) = info.video_decode_queue_family {
                ctx.qf[nb_qf as usize] = AVVulkanDeviceQueueFamily {
                    idx: vd as i32,
                    num: 1,
                    flags: vk::QueueFlags::VIDEO_DECODE_KHR.as_raw(),
                    video_caps: 0, // FFmpeg queries capabilities itself
                };
                nb_qf += 1;
            }

            ctx.nb_qf = nb_qf;
        }

        hw_device_ctx.init().map_err(|e| {
            VideoError::VulkanNotAvailable(format!("av_hwdevice_ctx_init failed: {e}"))
        })?;

        Ok(hw_device_ctx)
    }

    fn create_decoder(
        codecpar: &AVCodecParameters,
        hw_device_ctx: &AVHWDeviceContext,
    ) -> Result<AVCodecContext> {
        let codec_id = codecpar.codec_id;
        let codec = rsmpeg::avcodec::AVCodec::find_decoder(codec_id)
            .ok_or_else(|| VideoError::UnsupportedCodec(format!("codec id {codec_id}")))?;

        let mut decoder = AVCodecContext::new(&codec);

        let ret = unsafe {
            ffi::avcodec_parameters_to_context(decoder.as_mut_ptr(), codecpar.as_ptr())
        };
        if ret < 0 {
            return Err(VideoError::Decode(format!(
                "avcodec_parameters_to_context failed: {ret}"
            )));
        }

        unsafe {
            (*decoder.as_mut_ptr()).thread_count = 0;
            (*decoder.as_mut_ptr()).get_format = Some(vulkan_get_format);
        }

        decoder.set_hw_device_ctx(hw_device_ctx.clone());

        decoder.open(None).map_err(|e| {
            VideoError::VulkanNotAvailable(format!("failed to open Vulkan decoder: {e}"))
        })?;

        Ok(decoder)
    }
}

impl DecoderBackend for VulkanDecoder {
    fn send_packet(&mut self, packet: &rsmpeg::avcodec::AVPacket) -> Result<()> {
        self.decoder
            .send_packet(Some(packet))
            .map_err(|e| VideoError::Decode(format!("vulkan send_packet: {e}")))
    }

    fn send_eof(&mut self) -> Result<()> {
        self.decoder
            .send_packet(None)
            .map_err(|e| VideoError::Decode(format!("vulkan send_eof: {e}")))
    }

    fn decode_next(&mut self) -> Result<Option<VideoFrame>> {
        let frame = match self.decoder.receive_frame() {
            Ok(f) => f,
            Err(RsmpegError::DecoderDrainError) => return Ok(None),
            Err(RsmpegError::DecoderFlushedError) => return Ok(None),
            Err(e) => return Err(VideoError::Decode(format!("vulkan receive_frame: {e}"))),
        };

        let format = unsafe { (*frame.as_ptr()).format };
        if format != ffi::AV_PIX_FMT_VULKAN as i32 {
            // FFmpeg didn't produce Vulkan frames — fall back to CPU conversion
            return self.decode_sw_fallback(&frame);
        }

        let pts = pts_to_secs(frame.pts, self.time_base);
        log::trace!("Vulkan decoded frame: pts={pts:.3}, {}x{}", self.width, self.height);

        // data[0] points to AVVkFrame allocated by FFmpeg
        let vk_frame_ptr = unsafe { (*frame.as_ptr()).data[0] as *const AVVkFrame };
        if vk_frame_ptr.is_null() {
            return Err(VideoError::Decode("AVVkFrame pointer is null".into()));
        }

        let vk_frame = unsafe { &*vk_frame_ptr };

        // Wait for FFmpeg's decode to complete on the GPU (CPU-side wait on the
        // decode thread, so the render thread never blocks on this semaphore).
        let sem = vk_frame.sem[0];
        let sem_value = vk_frame.sem_value[0];
        if sem != vk::Semaphore::null() && sem_value > 0 {
            let sems = [sem];
            let values = [sem_value];
            let wait_info = vk::SemaphoreWaitInfo::default()
                .semaphores(&sems)
                .values(&values);
            let timeout_ns: u64 = 500_000_000;
            unsafe {
                match self.ash_device.wait_semaphores(&wait_info, timeout_ns) {
                    Ok(()) => {}
                    Err(vk::Result::TIMEOUT) => {
                        log::warn!("vkWaitSemaphores timed out (sem_value={sem_value})");
                    }
                    Err(e) => {
                        log::warn!("vkWaitSemaphores failed: {e:?}");
                    }
                }
            }
        }

        log::trace!(
            "AVVkFrame: img[0]={:?}, img[1]={:?}, layout[0]={}, layout[1]={}, sem={:?}, sem_val={}",
            vk_frame.img[0], vk_frame.img[1],
            vk_frame.layout[0], vk_frame.layout[1],
            vk_frame.sem[0], vk_frame.sem_value[0],
        );

        let gpu_frame = GpuFrame {
            images: [vk_frame.img[0], vk_frame.img[1]],
            layouts: [
                vk::ImageLayout::from_raw(vk_frame.layout[0] as i32),
                vk::ImageLayout::from_raw(vk_frame.layout[1] as i32),
            ],
            width: self.width,
            height: self.height,
            pts,
            _frame_ref: Arc::new(FrameRef { _frame: frame }),
        };

        Ok(Some(VideoFrame::Gpu(gpu_frame)))
    }

    fn flush(&mut self) {
        unsafe {
            ffi::avcodec_flush_buffers(self.decoder.as_mut_ptr());
        }
    }
}

impl VulkanDecoder {
    /// Fallback: convert a non-Vulkan frame to RGBA8 CPU frame via sws_scale.
    fn decode_sw_fallback(&mut self, frame: &AVFrame) -> Result<Option<VideoFrame>> {
        if !self.sw_warned {
            let format = unsafe { (*frame.as_ptr()).format };
            log::warn!(
                "Vulkan hwaccel not active (format={format}), falling back to CPU conversion"
            );
            self.sw_warned = true;
        }

        let src_format = unsafe { (*frame.as_ptr()).format };
        let w = self.width as i32;
        let h = self.height as i32;

        if self.sw_fallback.is_none() {
            self.sw_fallback = SwsContext::get_context(
                w, h, src_format, w, h,
                ffi::AV_PIX_FMT_RGBA,
                ffi::SWS_BILINEAR,
                None, None, None,
            );
            if self.sw_fallback.is_none() {
                return Err(VideoError::Decode(
                    "failed to create sws context for SW fallback".into(),
                ));
            }
        }

        let sws = self.sw_fallback.as_mut().unwrap();
        let pts = pts_to_secs(frame.pts, self.time_base);
        let cpu_frame = super::sws_frame_to_cpu(sws, frame, self.width, self.height, pts)?;
        Ok(Some(cpu_frame))
    }
}

/// `get_format` callback that tells FFmpeg to use Vulkan pixel format for hwaccel.
/// Without this, FFmpeg defaults to software formats (YUV420P).
unsafe extern "C" fn vulkan_get_format(
    _ctx: *mut ffi::AVCodecContext,
    pix_fmts: *const ffi::AVPixelFormat,
) -> ffi::AVPixelFormat {
    let mut formats = Vec::new();
    let mut i = 0;
    loop {
        let fmt = unsafe { *pix_fmts.add(i) };
        if fmt == ffi::AV_PIX_FMT_NONE {
            break;
        }
        formats.push(fmt);
        i += 1;
    }
    log::debug!("get_format: offered formats: {formats:?}");

    for &fmt in &formats {
        if fmt == ffi::AV_PIX_FMT_VULKAN {
            log::debug!("get_format: selected AV_PIX_FMT_VULKAN");
            return ffi::AV_PIX_FMT_VULKAN;
        }
    }
    // Return first format as fallback (let FFmpeg pick software format)
    let fallback = formats.first().copied().unwrap_or(ffi::AV_PIX_FMT_NONE);
    log::debug!("get_format: AV_PIX_FMT_VULKAN not offered, using fallback format {fallback}");
    fallback
}
