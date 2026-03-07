mod effect;
mod grayscale;

pub use effect::PostEffect;
pub use grayscale::Grayscale;

use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::error::Result;
use crate::frame::MAX_FRAMES_IN_FLIGHT;
use crate::gpu::GpuContext;
use crate::resource::{Buffer, Image};

const COLOR_SUBRESOURCE_RANGE: vk::ImageSubresourceRange = vk::ImageSubresourceRange {
    aspect_mask: vk::ImageAspectFlags::COLOR,
    base_mip_level: 0,
    level_count: 1,
    base_array_layer: 0,
    layer_count: 1,
};

pub struct PostEffectChain {
    effects: Vec<Box<dyn PostEffect>>,
    ping: Option<Image>,
    pong: Option<Image>,
    current_extent: vk::Extent2D,
    sampler: vk::Sampler,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    // [frame_index][direction]: direction 0 = ping→pong, direction 1 = pong→ping
    descriptor_sets: Vec<[vk::DescriptorSet; 2]>,
    mask_view: Option<vk::ImageView>,
    mask_image: Option<Image>,
    dummy_mask: Option<Image>,
    device: ash::Device,
}

impl PostEffectChain {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
        let device = gpu.ash_device();

        // LINEAR sampler with CLAMP_TO_EDGE
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::LINEAR)
            .min_filter(vk::Filter::LINEAR)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        // Shared descriptor set layout
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];
        let layout_info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        // Descriptor pool: 2 directions × MAX_FRAMES_IN_FLIGHT
        let total_sets = 2 * MAX_FRAMES_IN_FLIGHT as u32;
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(2 * total_sets), // binding 0 + binding 2
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(total_sets), // binding 1
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(total_sets)
            .pool_sizes(&pool_sizes);
        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        // Allocate descriptor sets
        let set_layouts: Vec<_> = (0..total_sets).map(|_| descriptor_set_layout).collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let all_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        let mut descriptor_sets = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            descriptor_sets.push([all_sets[i * 2], all_sets[i * 2 + 1]]);
        }

        // 1×1 white dummy mask
        let dummy_mask = create_dummy_mask(gpu)?;

        Ok(Self {
            effects: Vec::new(),
            ping: None,
            pong: None,
            current_extent: vk::Extent2D { width: 0, height: 0 },
            sampler,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            mask_view: None,
            mask_image: None,
            dummy_mask: Some(dummy_mask),
            device: device.clone(),
        })
    }

    pub fn add(&mut self, gpu: &GpuContext, mut effect: Box<dyn PostEffect>) -> Result<()> {
        let name = effect.name();
        if self.effects.iter().any(|e| e.name() == name) {
            return Err(crate::error::RendererError::PostEffect(format!(
                "effect '{}' already exists in chain",
                name,
            )));
        }
        effect.initialize(gpu, self.descriptor_set_layout)?;
        self.effects.push(effect);
        Ok(())
    }

    pub fn remove(&mut self, name: &str) {
        if let Some(pos) = self.effects.iter().position(|e| e.name() == name) {
            unsafe {
                let _ = self.device.device_wait_idle();
            }
            let mut effect = self.effects.remove(pos);
            effect.destroy(&self.device);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.effects.is_empty()
    }

    pub fn set_mask(&mut self, view: vk::ImageView) {
        self.mask_image = None;
        self.mask_view = Some(view);
        if self.ping.is_some() {
            self.update_descriptor_sets();
        }
    }

    pub fn set_mask_from_data(
        &mut self,
        gpu: &GpuContext,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<()> {
        let image = upload_r8_image(gpu, width, height, data)?;
        let view = image.view();
        self.mask_image = Some(image);
        self.mask_view = Some(view);
        if self.ping.is_some() {
            self.update_descriptor_sets();
        }
        Ok(())
    }

    pub fn clear_mask(&mut self) {
        self.mask_view = None;
        self.mask_image = None;
        if self.ping.is_some() {
            self.update_descriptor_sets();
        }
    }

    pub fn get<T: 'static>(&self, name: &str) -> Option<&T> {
        self.effects
            .iter()
            .find(|e| e.name() == name)
            .and_then(|e| e.as_any().downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static>(&mut self, name: &str) -> Option<&mut T> {
        self.effects
            .iter_mut()
            .find(|e| e.name() == name)
            .and_then(|e| e.as_any_mut().downcast_mut::<T>())
    }

    fn ensure_images(&mut self, gpu: &GpuContext, extent: vk::Extent2D) -> Result<()> {
        if self.current_extent == extent && self.ping.is_some() {
            return Ok(());
        }

        // Drop old images
        self.ping = None;
        self.pong = None;

        let usage = vk::ImageUsageFlags::STORAGE
            | vk::ImageUsageFlags::SAMPLED
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::TRANSFER_SRC;

        self.ping = Some(Image::new(
            gpu,
            extent.width,
            extent.height,
            vk::Format::R8G8B8A8_UNORM,
            usage,
            MemoryLocation::GpuOnly,
        )?);
        self.pong = Some(Image::new(
            gpu,
            extent.width,
            extent.height,
            vk::Format::R8G8B8A8_UNORM,
            usage,
            MemoryLocation::GpuOnly,
        )?);

        self.current_extent = extent;
        self.update_descriptor_sets();
        Ok(())
    }

    fn update_descriptor_sets(&self) {
        let ping = self.ping.as_ref().unwrap();
        let pong = self.pong.as_ref().unwrap();
        let mask_view = self
            .mask_view
            .unwrap_or_else(|| self.dummy_mask.as_ref().unwrap().view());

        for frame_idx in 0..MAX_FRAMES_IN_FLIGHT {
            let sets = &self.descriptor_sets[frame_idx];

            // Direction 0: read ping → write pong
            self.write_descriptor_set(sets[0], ping.view(), pong.view(), mask_view);
            // Direction 1: read pong → write ping
            self.write_descriptor_set(sets[1], pong.view(), ping.view(), mask_view);
        }
    }

    fn write_descriptor_set(
        &self,
        set: vk::DescriptorSet,
        input_view: vk::ImageView,
        output_view: vk::ImageView,
        mask_view: vk::ImageView,
    ) {
        let input_info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(input_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let output_info = vk::DescriptorImageInfo::default()
            .image_view(output_view)
            .image_layout(vk::ImageLayout::GENERAL);

        let mask_info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(mask_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&input_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&output_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&mask_info)),
        ];

        unsafe {
            self.device.update_descriptor_sets(&writes, &[]);
        }
    }

    pub fn apply(
        &mut self,
        gpu: &GpuContext,
        cmd: vk::CommandBuffer,
        swapchain_image: vk::Image,
        extent: vk::Extent2D,
        frame_index: usize,
    ) -> Result<()> {
        if self.effects.is_empty() {
            return Ok(());
        }

        self.ensure_images(gpu, extent)?;

        let device = gpu.ash_device();
        let ping = self.ping.as_ref().unwrap();
        let pong = self.pong.as_ref().unwrap();

        unsafe {
            // 1+2. Swapchain → TRANSFER_SRC, Ping → TRANSFER_DST (batched)
            {
                let barriers = [
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
                        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .image(swapchain_image)
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                        .src_access_mask(vk::AccessFlags2::NONE)
                        .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .image(ping.raw())
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                ];
                let dep = vk::DependencyInfo::default().image_memory_barriers(&barriers);
                device.cmd_pipeline_barrier2(cmd, &dep);
            }

            // 3. Blit swapchain → ping
            let blit_region = vk::ImageBlit {
                src_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                src_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: extent.width as i32,
                        y: extent.height as i32,
                        z: 1,
                    },
                ],
                dst_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: 1,
                },
                dst_offsets: [
                    vk::Offset3D { x: 0, y: 0, z: 0 },
                    vk::Offset3D {
                        x: extent.width as i32,
                        y: extent.height as i32,
                        z: 1,
                    },
                ],
            };

            device.cmd_blit_image(
                cmd,
                swapchain_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                ping.raw(),
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[blit_region],
                vk::Filter::NEAREST,
            );

            // 4+5a. Ping: TRANSFER_DST → SHADER_READ_ONLY + Pong: UNDEFINED → GENERAL (batched)
            {
                let barriers = [
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::SHADER_READ)
                        .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .image(ping.raw())
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                        .src_access_mask(vk::AccessFlags2::NONE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .dst_access_mask(vk::AccessFlags2::SHADER_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::GENERAL)
                        .image(pong.raw())
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                ];
                let dep = vk::DependencyInfo::default().image_memory_barriers(&barriers);
                device.cmd_pipeline_barrier2(cmd, &dep);
            }

            // 5. Process effects with ping-pong
            // direction: 0 = read ping, write pong; 1 = read pong, write ping
            let mut direction: usize = 0;
            let mut first_pass = true;

            for effect in &self.effects {
                for pass_idx in 0..effect.pass_count() {
                    let output_image = if direction == 0 {
                        pong.raw()
                    } else {
                        ping.raw()
                    };

                    // First pass's output (pong) was already transitioned in the batched barrier above
                    if first_pass {
                        first_pass = false;
                    } else {
                        self.barrier(
                            device,
                            cmd,
                            output_image,
                            vk::PipelineStageFlags2::TOP_OF_PIPE,
                            vk::AccessFlags2::NONE,
                            vk::PipelineStageFlags2::COMPUTE_SHADER,
                            vk::AccessFlags2::SHADER_WRITE,
                            vk::ImageLayout::UNDEFINED,
                            vk::ImageLayout::GENERAL,
                        );
                    }

                    let desc_set = self.descriptor_sets[frame_index][direction];
                    effect.record(device, cmd, pass_idx, desc_set, extent);

                    // Output: GENERAL → SHADER_READ_ONLY_OPTIMAL
                    self.barrier(
                        device,
                        cmd,
                        output_image,
                        vk::PipelineStageFlags2::COMPUTE_SHADER,
                        vk::AccessFlags2::SHADER_WRITE,
                        vk::PipelineStageFlags2::COMPUTE_SHADER,
                        vk::AccessFlags2::SHADER_READ,
                        vk::ImageLayout::GENERAL,
                        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
                    );

                    // Swap direction
                    direction = 1 - direction;
                }
            }

            // The result is the last written image (which is now the "input" after direction swap)
            // direction was swapped after the last pass, so the result is in:
            // direction=0 → result in ping (was written when direction was 1)
            // direction=1 → result in pong (was written when direction was 0)
            let result_image = if direction == 0 { ping.raw() } else { pong.raw() };

            // 6+7. Result → TRANSFER_SRC, Swapchain → TRANSFER_DST (batched)
            {
                let barriers = [
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
                        .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_READ)
                        .old_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .image(result_image)
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .src_access_mask(vk::AccessFlags2::TRANSFER_READ)
                        .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
                        .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
                        .old_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                        .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                        .image(swapchain_image)
                        .subresource_range(COLOR_SUBRESOURCE_RANGE),
                ];
                let dep = vk::DependencyInfo::default().image_memory_barriers(&barriers);
                device.cmd_pipeline_barrier2(cmd, &dep);
            }

            // 8. Blit result → swapchain
            device.cmd_blit_image(
                cmd,
                result_image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                swapchain_image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &[blit_region],
                vk::Filter::NEAREST,
            );

            // 9. Swapchain: TRANSFER_DST → PRESENT_SRC_KHR
            self.barrier(
                device,
                cmd,
                swapchain_image,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
                vk::AccessFlags2::NONE,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                vk::ImageLayout::PRESENT_SRC_KHR,
            );
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn barrier(
        &self,
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        image: vk::Image,
        src_stage: vk::PipelineStageFlags2,
        src_access: vk::AccessFlags2,
        dst_stage: vk::PipelineStageFlags2,
        dst_access: vk::AccessFlags2,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(src_stage)
            .src_access_mask(src_access)
            .dst_stage_mask(dst_stage)
            .dst_access_mask(dst_access)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .image(image)
            .subresource_range(COLOR_SUBRESOURCE_RANGE);

        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier));
        unsafe { device.cmd_pipeline_barrier2(cmd, &dep) };
    }

    pub fn destroy(&mut self, gpu: &GpuContext) {
        let device = gpu.ash_device();
        for effect in &mut self.effects {
            effect.destroy(device);
        }
        self.effects.clear();

        self.ping = None;
        self.pong = None;
        self.mask_view = None;
        self.mask_image = None;
        self.dummy_mask = None;

        unsafe {
            device.destroy_sampler(self.sampler, None);
            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

fn upload_r8_image(gpu: &GpuContext, width: u32, height: u32, data: &[u8]) -> Result<Image> {
    assert_eq!(data.len(), (width * height) as usize);

    let image = Image::new(
        gpu,
        width,
        height,
        vk::Format::R8_UNORM,
        vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
        MemoryLocation::GpuOnly,
    )?;

    let mut staging = Buffer::new(
        gpu,
        data.len() as u64,
        vk::BufferUsageFlags::TRANSFER_SRC,
        MemoryLocation::CpuToGpu,
    )?;
    if let Some(mapped) = staging.mapped_slice_mut() {
        mapped[..data.len()].copy_from_slice(data);
    }

    let device = gpu.ash_device();
    let staging_raw = staging.raw();

    gpu.submit_oneshot(|cmd| unsafe {
        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .image(image.raw())
            .subresource_range(COLOR_SUBRESOURCE_RANGE);
        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier));
        device.cmd_pipeline_barrier2(cmd, &dep);

        let region = vk::BufferImageCopy::default()
            .image_subresource(vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: 1,
            })
            .image_extent(vk::Extent3D {
                width,
                height,
                depth: 1,
            });
        device.cmd_copy_buffer_to_image(
            cmd,
            staging_raw,
            image.raw(),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &[region],
        );

        let barrier = vk::ImageMemoryBarrier2::default()
            .src_stage_mask(vk::PipelineStageFlags2::TRANSFER)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_stage_mask(vk::PipelineStageFlags2::COMPUTE_SHADER)
            .dst_access_mask(vk::AccessFlags2::SHADER_READ)
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image(image.raw())
            .subresource_range(COLOR_SUBRESOURCE_RANGE);
        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(std::slice::from_ref(&barrier));
        device.cmd_pipeline_barrier2(cmd, &dep);
    })?;

    drop(staging);

    Ok(image)
}

fn create_dummy_mask(gpu: &GpuContext) -> Result<Image> {
    upload_r8_image(gpu, 1, 1, &[255])
}
