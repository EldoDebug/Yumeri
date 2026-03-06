pub mod yuv_converter;

use ash::vk;
use gpu_allocator::MemoryLocation;
use yumeri_video::{GpuFrame, VideoFrame, VideoHandle};

use crate::error::Result;
use crate::frame::MAX_FRAMES_IN_FLIGHT;
use crate::gpu::GpuContext;
use crate::resource::{create_image_view, Buffer};
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

use self::yuv_converter::YuvConverter;

/// Keeps a GPU-decoded frame and its VkImageViews alive until the GPU is done.
struct GpuFrameSlot {
    _gpu_frame: GpuFrame,
    luma_view: vk::ImageView,
    chroma_view: vk::ImageView,
}

/// Manages a video stream's texture with streaming GPU uploads.
///
/// Supports two paths:
/// - **CPU frames**: staging buffer upload (software decode fallback)
/// - **GPU frames**: NV12→RGBA8 compute shader conversion (Vulkan hwaccel zero-copy)
pub struct VideoTexture {
    texture_id: TextureId,
    handle: VideoHandle,
    last_pts: f64,
    // CPU frame path
    staging_buffers: Vec<Buffer>,
    pending_upload: bool,
    width: u32,
    height: u32,
    // GPU frame path
    yuv_converter: Option<YuvConverter>,
    gpu_frame_slots: Vec<Option<GpuFrameSlot>>,
    pending_gpu_frame: Option<GpuFrame>,
    device: ash::Device,
}

impl VideoTexture {
    pub fn new(
        gpu: &GpuContext,
        store: &mut TextureStore,
        handle: VideoHandle,
    ) -> Result<Self> {
        let w = handle.width();
        let h = handle.height();
        let byte_size = (w * h * 4) as u64;

        let black = vec![0u8; byte_size as usize];
        let texture_id = store.create_from_raw_rgba(gpu, w, h, &black)?;

        let mut staging_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            staging_buffers.push(Buffer::new(
                gpu,
                byte_size,
                vk::BufferUsageFlags::TRANSFER_SRC,
                MemoryLocation::CpuToGpu,
            )?);
        }

        Ok(Self {
            texture_id,
            handle,
            last_pts: -1.0,
            staging_buffers,
            pending_upload: false,
            width: w,
            height: h,
            yuv_converter: None,
            gpu_frame_slots: (0..MAX_FRAMES_IN_FLIGHT).map(|_| None).collect(),
            pending_gpu_frame: None,
            device: gpu.ash_device().clone(),
        })
    }

    /// Drain decoded frames and stage the latest for GPU upload.
    /// CPU-only for CPU frames (memcpy to staging buffer).
    /// For GPU frames, stores the frame for later compute shader conversion.
    pub fn update(&mut self, frame_index: usize) {
        let mut latest = None;
        while let Some(frame) = self.handle.next_frame() {
            latest = Some(frame);
        }

        if let Some(frame) = latest {
            let pts = frame.pts();
            if pts <= self.last_pts && pts > 0.0 {
                return;
            }

            match frame {
                VideoFrame::Cpu {
                    ref data,
                    width,
                    height,
                    format,
                    ..
                } => {
                    if format != yumeri_video::VideoPixelFormat::Rgba8 {
                        log::warn!("Non-RGBA8 CPU frame ({format:?}), skipping");
                        return;
                    }
                    if width != self.width || height != self.height {
                        log::warn!(
                            "Resolution changed ({width}x{height} vs {}x{}), frame skipped",
                            self.width, self.height
                        );
                        return;
                    }
                    let buf_idx = frame_index % self.staging_buffers.len();
                    let staging = &mut self.staging_buffers[buf_idx];
                    if let Some(mapped) = staging.mapped_slice_mut() {
                        let len = data.len().min(mapped.len());
                        mapped[..len].copy_from_slice(&data[..len]);
                        self.pending_upload = true;
                        self.pending_gpu_frame = None;
                        self.last_pts = pts;
                    }
                }
                VideoFrame::Gpu(gpu_frame) => {
                    log::trace!("VideoTexture: received GPU frame pts={pts:.3}");
                    self.pending_gpu_frame = Some(gpu_frame);
                    self.pending_upload = false;
                    self.last_pts = pts;
                }
            }
        }
    }

    /// Record GPU commands for the pending frame.
    /// For CPU frames: staging buffer → image copy.
    /// For GPU frames: NV12 → RGBA8 compute shader dispatch.
    pub fn record_upload(
        &mut self,
        cmd: vk::CommandBuffer,
        gpu: &GpuContext,
        store: &mut TextureStore,
        frame_index: usize,
    ) {
        if self.pending_upload {
            let buf_idx = frame_index % self.staging_buffers.len();
            let staging_raw = self.staging_buffers[buf_idx].raw();
            store.record_image_upload(cmd, self.texture_id, staging_raw, self.width, self.height);
            self.pending_upload = false;
            return;
        }

        if let Some(gpu_frame) = self.pending_gpu_frame.take() {
            log::trace!("VideoTexture: recording GPU convert (frame_index={frame_index})");
            if let Err(e) = self.record_gpu_convert(cmd, gpu, store, gpu_frame, frame_index) {
                log::error!("GPU frame conversion failed: {e}");
            }
        }
    }

    fn record_gpu_convert(
        &mut self,
        cmd: vk::CommandBuffer,
        gpu: &GpuContext,
        store: &mut TextureStore,
        gpu_frame: GpuFrame,
        frame_index: usize,
    ) -> Result<()> {
        let device = gpu.ash_device();
        let slot_idx = frame_index % MAX_FRAMES_IN_FLIGHT;

        // Destroy old views from this slot (safe: frame-in-flight guarantees GPU is done)
        if let Some(old_slot) = self.gpu_frame_slots[slot_idx].take() {
            unsafe {
                device.destroy_image_view(old_slot.luma_view, None);
                device.destroy_image_view(old_slot.chroma_view, None);
            }
        }

        // Semaphore wait already done by VulkanDecoder in the decode thread.
        let images = gpu_frame.images();
        let layouts = gpu_frame.layouts();
        let width = gpu_frame.width();
        let height = gpu_frame.height();

        // FFmpeg may produce either:
        // - Two separate VkImages (img[0]=luma, img[1]=chroma)
        // - One multiplane VkImage (img[0]=multiplane, img[1]=null)
        let multiplane = images[1] == vk::Image::null();

        let (luma_view, chroma_view) = if multiplane {
            // Multiplane: create views from the same image using plane aspect bits
            let luma = create_image_view(
                device,
                images[0],
                vk::Format::R8_UNORM,
                vk::ImageAspectFlags::PLANE_0,
            )?;
            let chroma = create_image_view(
                device,
                images[0],
                vk::Format::R8G8_UNORM,
                vk::ImageAspectFlags::PLANE_1,
            )?;
            (luma, chroma)
        } else {
            // Separate images: use COLOR aspect
            let luma = create_image_view(
                device,
                images[0],
                vk::Format::R8_UNORM,
                vk::ImageAspectFlags::COLOR,
            )?;
            let chroma = create_image_view(
                device,
                images[1],
                vk::Format::R8G8_UNORM,
                vk::ImageAspectFlags::COLOR,
            )?;
            (luma, chroma)
        };

        // Transition NV12 planes to SHADER_READ_ONLY_OPTIMAL for compute shader sampling
        unsafe {
            let make_barrier = |image, old_layout, aspect_mask| {
                vk::ImageMemoryBarrier::default()
                    .old_layout(old_layout)
                    .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                    .image(image)
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                    .dst_access_mask(vk::AccessFlags::SHADER_READ)
            };

            let (barriers, count) = if multiplane {
                // Multiplane NV12: single barrier covering both planes
                let aspect = vk::ImageAspectFlags::PLANE_0 | vk::ImageAspectFlags::PLANE_1;
                ([make_barrier(images[0], layouts[0], aspect), Default::default()], 1)
            } else {
                // Separate images: use COLOR aspect for each
                ([
                    make_barrier(images[0], layouts[0], vk::ImageAspectFlags::COLOR),
                    make_barrier(images[1], layouts[1], vk::ImageAspectFlags::COLOR),
                ], 2)
            };

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::ALL_COMMANDS,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers[..count],
            );
        }

        if self.yuv_converter.is_none() {
            self.yuv_converter = Some(YuvConverter::new(gpu)?);
        }

        let converter = self.yuv_converter.as_mut().unwrap();
        let srgb_view = converter.convert(
            gpu,
            cmd,
            luma_view,
            chroma_view,
            width,
            height,
            frame_index,
        )?;

        // Point the texture store's descriptor at the converter's SRGB output view
        store.set_override_view(self.texture_id, srgb_view);

        // Store slot to keep GpuFrame (and its AVFrame/VkImages) alive
        self.gpu_frame_slots[slot_idx] = Some(GpuFrameSlot {
            _gpu_frame: gpu_frame,
            luma_view,
            chroma_view,
        });

        Ok(())
    }

    pub fn texture_id(&self) -> TextureId {
        self.texture_id
    }

    pub fn handle(&self) -> &VideoHandle {
        &self.handle
    }

    pub fn destroy(&mut self, store: &mut TextureStore) {
        for slot in self.gpu_frame_slots.iter_mut() {
            if let Some(s) = slot.take() {
                unsafe {
                    self.device.destroy_image_view(s.luma_view, None);
                    self.device.destroy_image_view(s.chroma_view, None);
                }
            }
        }
        if let Some(converter) = &mut self.yuv_converter {
            converter.destroy();
        }
        self.staging_buffers.clear();
        store.remove(self.texture_id);
    }
}
