use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use ash::vk;
use gpu_allocator::MemoryLocation;
use slotmap::SlotMap;

use super::gpu_texture::GpuTexture;
use super::TextureId;
use crate::error::{RendererError, Result};
use crate::frame::MAX_FRAMES_IN_FLIGHT;
use crate::gpu::GpuContext;
use crate::resource::{Buffer, Image};

const MAX_TEXTURES: u32 = 512;

fn ensure_rgba8(img: &yumeri_image::Image) -> std::result::Result<yumeri_image::Image, RendererError> {
    if img.format() == yumeri_image::PixelFormat::Rgba8 {
        Ok(img.clone())
    } else {
        img.convert_to(yumeri_image::PixelFormat::Rgba8)
            .map_err(|e| RendererError::Texture(e.to_string()))
    }
}

pub struct TextureStore {
    textures: SlotMap<TextureId, GpuTexture>,
    path_cache: HashMap<PathBuf, TextureId>,
    free_indices: Vec<u32>,
    next_descriptor_index: u32,

    default_sampler: vk::Sampler,
    placeholder_id: Option<TextureId>,

    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_sets: Vec<vk::DescriptorSet>,
    dirty: bool,

    device: ash::Device,

    // Retired images kept alive until descriptors are flushed
    retired_images: Vec<Image>,

    receiver: mpsc::Receiver<(TextureId, std::result::Result<yumeri_image::Image, String>)>,
    sender: mpsc::Sender<(TextureId, std::result::Result<yumeri_image::Image, String>)>,
}

impl TextureStore {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
        let device = gpu.ash_device();

        let binding_flags = [vk::DescriptorBindingFlags::PARTIALLY_BOUND
            | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND];
        let mut binding_flags_info =
            vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&binding_flags);

        let binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(MAX_TEXTURES)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT);

        let layout_info = vk::DescriptorSetLayoutCreateInfo::default()
            .bindings(std::slice::from_ref(&binding))
            .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .push_next(&mut binding_flags_info);

        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .descriptor_count(MAX_TEXTURES * MAX_FRAMES_IN_FLIGHT as u32);

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(MAX_FRAMES_IN_FLIGHT as u32)
            .pool_sizes(std::slice::from_ref(&pool_size))
            .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND);

        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        let layouts: Vec<_> = (0..MAX_FRAMES_IN_FLIGHT)
            .map(|_| descriptor_set_layout)
            .collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        let default_sampler = create_sampler(gpu)?;

        let (sender, receiver) = mpsc::channel();

        let mut store = Self {
            textures: SlotMap::with_key(),
            path_cache: HashMap::new(),
            free_indices: Vec::new(),
            next_descriptor_index: 0,
            default_sampler,
            placeholder_id: None,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_sets,
            dirty: false,
            device: device.clone(),
            retired_images: Vec::new(),
            receiver,
            sender,
        };

        store.create_placeholder(gpu)?;

        Ok(store)
    }

    fn allocate_descriptor_index(&mut self) -> Result<u32> {
        if let Some(idx) = self.free_indices.pop() {
            return Ok(idx);
        }
        if self.next_descriptor_index >= MAX_TEXTURES {
            return Err(RendererError::Texture(format!(
                "maximum texture count ({MAX_TEXTURES}) exceeded"
            )));
        }
        let idx = self.next_descriptor_index;
        self.next_descriptor_index += 1;
        Ok(idx)
    }

    fn free_descriptor_index(&mut self, idx: u32) {
        self.free_indices.push(idx);
    }

    fn create_placeholder(&mut self, gpu: &GpuContext) -> Result<()> {
        let white_pixel: [u8; 4] = [255, 255, 255, 255];
        let image = upload_image_to_gpu(gpu, 1, 1, &white_pixel)?;

        let desc_idx = self.allocate_descriptor_index()?;

        let gpu_tex = GpuTexture {
            image,
            sampler: self.default_sampler,
            descriptor_index: desc_idx,
        };

        let id = self.textures.insert(gpu_tex);
        self.placeholder_id = Some(id);
        self.dirty = true;
        Ok(())
    }

    pub fn create(
        &mut self,
        gpu: &GpuContext,
        img: &yumeri_image::Image,
    ) -> Result<TextureId> {
        let rgba = ensure_rgba8(img)?;
        let image = upload_image_to_gpu(gpu, rgba.width(), rgba.height(), rgba.data())?;

        let desc_idx = self.allocate_descriptor_index()?;

        let gpu_tex = GpuTexture {
            image,
            sampler: self.default_sampler,
            descriptor_index: desc_idx,
        };

        let id = self.textures.insert(gpu_tex);
        self.dirty = true;
        Ok(id)
    }

    pub fn load(&mut self, gpu: &GpuContext, path: impl Into<PathBuf>) -> TextureId {
        let path = path.into();
        if let Some(&id) = self.path_cache.get(&path) {
            return id;
        }

        let desc_idx = self
            .allocate_descriptor_index()
            .expect("texture limit exceeded during load");

        // Create a placeholder entry that shares the placeholder's view via a 1x1 upload.
        // We create a minimal image so this entry owns its own Image resource for clean drop.
        let placeholder_image =
            upload_image_to_gpu(gpu, 1, 1, &[255, 255, 255, 255]).expect("placeholder upload");

        let gpu_tex = GpuTexture {
            image: placeholder_image,
            sampler: self.default_sampler,
            descriptor_index: desc_idx,
        };

        let id = self.textures.insert(gpu_tex);
        self.path_cache.insert(path.clone(), id);
        self.dirty = true;

        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = yumeri_image::Image::load(&path).map_err(|e| e.to_string());
            let _ = sender.send((id, result));
        });

        id
    }

    pub fn process_pending(&mut self, gpu: &GpuContext) {
        while let Ok((id, result)) = self.receiver.try_recv() {
            let Ok(img) = result else {
                log::error!("Failed to load texture: {}", result.unwrap_err());
                continue;
            };

            let rgba = match ensure_rgba8(&img) {
                Ok(converted) => converted,
                Err(e) => {
                    log::error!("Failed to convert texture to RGBA8: {e}");
                    continue;
                }
            };

            let image = match upload_image_to_gpu(gpu, rgba.width(), rgba.height(), rgba.data()) {
                Ok(img) => img,
                Err(e) => {
                    log::error!("Failed to upload texture to GPU: {e}");
                    continue;
                }
            };

            if let Some(gpu_tex) = self.textures.get_mut(id) {
                let old_image = std::mem::replace(&mut gpu_tex.image, image);
                self.retired_images.push(old_image);
                self.dirty = true;
            }
        }
    }

    pub fn flush_descriptors(&mut self, _frame_index: usize) {
        if !self.dirty {
            return;
        }

        // Collect all image infos and descriptor indices first
        let entries: Vec<(u32, vk::DescriptorImageInfo)> = self
            .textures
            .iter()
            .map(|(_, gpu_tex)| {
                (
                    gpu_tex.descriptor_index,
                    vk::DescriptorImageInfo::default()
                        .sampler(gpu_tex.sampler)
                        .image_view(gpu_tex.image.view())
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL),
                )
            })
            .collect();

        for &set in &self.descriptor_sets {
            // Batch all writes for this set
            let writes: Vec<vk::WriteDescriptorSet> = entries
                .iter()
                .map(|(desc_idx, info)| {
                    vk::WriteDescriptorSet::default()
                        .dst_set(set)
                        .dst_binding(0)
                        .dst_array_element(*desc_idx)
                        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                        .image_info(std::slice::from_ref(info))
                })
                .collect();

            unsafe {
                self.device.update_descriptor_sets(&writes, &[]);
            }
        }

        // Now safe to drop retired images - all descriptor sets have been updated
        self.retired_images.clear();
        self.dirty = false;
    }

    pub fn resolve(&self, id: TextureId) -> u32 {
        self.textures
            .get(id)
            .map(|t| t.descriptor_index)
            .unwrap_or(0)
    }

    pub fn remove(&mut self, id: TextureId) {
        self.path_cache.retain(|_, v| *v != id);
        if let Some(gpu_tex) = self.textures.remove(id) {
            self.free_descriptor_index(gpu_tex.descriptor_index);
            // Don't destroy default_sampler as it's shared
            if gpu_tex.sampler != self.default_sampler {
                unsafe {
                    self.device.destroy_sampler(gpu_tex.sampler, None);
                }
            }
        }
    }

    pub fn descriptor_set_layout(&self) -> vk::DescriptorSetLayout {
        self.descriptor_set_layout
    }

    pub fn descriptor_set(&self, frame_index: usize) -> vk::DescriptorSet {
        self.descriptor_sets[frame_index]
    }

    pub fn destroy(&mut self) {
        self.retired_images.clear();
        self.textures.clear();
        unsafe {
            self.device.destroy_sampler(self.default_sampler, None);
            if self.descriptor_pool != vk::DescriptorPool::null() {
                self.device
                    .destroy_descriptor_pool(self.descriptor_pool, None);
            }
            if self.descriptor_set_layout != vk::DescriptorSetLayout::null() {
                self.device
                    .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            }
        }
    }
}

fn upload_image_to_gpu(gpu: &GpuContext, width: u32, height: u32, data: &[u8]) -> Result<Image> {
    let byte_size = (width * height * 4) as u64;

    let mut staging = Buffer::new(
        gpu,
        byte_size,
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
    )?;

    if let Some(mapped) = staging.mapped_slice_mut() {
        mapped[..data.len()].copy_from_slice(data);
    }

    let image = Image::new(
        gpu,
        width,
        height,
        vk::Format::R8G8B8A8_SRGB,
        vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
        MemoryLocation::GpuOnly,
    )?;

    gpu.submit_oneshot(|cmd| {
        let device = gpu.ash_device();
        unsafe {
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image.raw())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            let region = vk::BufferImageCopy::default()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(vk::Extent3D {
                    width,
                    height,
                    depth: 1,
                });

            device.cmd_copy_buffer_to_image(
                cmd,
                staging.raw(),
                image.raw(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[region],
            );

            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(image.raw())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }
    })?;

    Ok(image)
}

fn create_sampler(gpu: &GpuContext) -> Result<vk::Sampler> {
    let sampler_info = vk::SamplerCreateInfo::default()
        .mag_filter(vk::Filter::LINEAR)
        .min_filter(vk::Filter::LINEAR)
        .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .mipmap_mode(vk::SamplerMipmapMode::LINEAR)
        .max_lod(1.0);

    let sampler = unsafe { gpu.ash_device().create_sampler(&sampler_info, None)? };
    Ok(sampler)
}
