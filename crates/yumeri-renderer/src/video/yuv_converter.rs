use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::error::{RendererError, Result};
use crate::gpu::GpuContext;
use crate::resource::Image;

const OUTPUT_RING_SIZE: usize = 2;

struct OutputSlot {
    image: Option<Image>,
    dims: (u32, u32),
    srgb_view: Option<vk::ImageView>,
}

/// Compute pipeline for converting NV12 VkImages to RGBA8.
pub struct YuvConverter {
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    sampler: vk::Sampler,
    output_slots: Vec<OutputSlot>,
    device: ash::Device,
}

impl YuvConverter {
    pub fn new(gpu: &GpuContext) -> Result<Self> {
        let device = gpu.ash_device();

        // Descriptor set layout: 2 input samplers + 1 storage image
        let bindings = [
            vk::DescriptorSetLayoutBinding::default()
                .binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
            vk::DescriptorSetLayoutBinding::default()
                .binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::COMPUTE),
        ];

        let layout_info =
            vk::DescriptorSetLayoutCreateInfo::default().bindings(&bindings);
        let descriptor_set_layout =
            unsafe { device.create_descriptor_set_layout(&layout_info, None)? };

        // Pipeline layout
        let layouts = [descriptor_set_layout];
        let pipeline_layout_info =
            vk::PipelineLayoutCreateInfo::default().set_layouts(&layouts);
        let pipeline_layout =
            unsafe { device.create_pipeline_layout(&pipeline_layout_info, None)? };

        // Load compute shader
        let shader_bytes =
            include_bytes!(concat!(env!("OUT_DIR"), "/yuv_to_rgb.comp.spv"));
        let shader_words: Vec<u32> = shader_bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();

        let shader_module_info =
            vk::ShaderModuleCreateInfo::default().code(&shader_words);
        let shader_module =
            unsafe { device.create_shader_module(&shader_module_info, None)? };

        let entry_point = c"main";
        let stage = vk::PipelineShaderStageCreateInfo::default()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(shader_module)
            .name(entry_point);

        let pipeline_info = vk::ComputePipelineCreateInfo::default()
            .stage(stage)
            .layout(pipeline_layout);

        let pipeline = unsafe {
            device
                .create_compute_pipelines(vk::PipelineCache::null(), &[pipeline_info], None)
                .map_err(|(_pipelines, err)| RendererError::Vulkan(err))?[0]
        };

        unsafe {
            device.destroy_shader_module(shader_module, None);
        }

        // Descriptor pool
        let pool_sizes = [
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(2 * OUTPUT_RING_SIZE as u32),
            vk::DescriptorPoolSize::default()
                .ty(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(OUTPUT_RING_SIZE as u32),
        ];
        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(OUTPUT_RING_SIZE as u32)
            .pool_sizes(&pool_sizes);
        let descriptor_pool =
            unsafe { device.create_descriptor_pool(&pool_info, None)? };

        let set_layouts: Vec<_> = (0..OUTPUT_RING_SIZE)
            .map(|_| descriptor_set_layout)
            .collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&set_layouts);
        let descriptor_sets =
            unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        // Nearest-neighbor sampler for YUV planes
        let sampler_info = vk::SamplerCreateInfo::default()
            .mag_filter(vk::Filter::NEAREST)
            .min_filter(vk::Filter::NEAREST)
            .address_mode_u(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_v(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .address_mode_w(vk::SamplerAddressMode::CLAMP_TO_EDGE);
        let sampler = unsafe { device.create_sampler(&sampler_info, None)? };

        Ok(Self {
            pipeline,
            pipeline_layout,
            descriptor_set_layout,
            descriptor_pool,
            descriptor_sets,
            sampler,
            output_slots: (0..OUTPUT_RING_SIZE)
                .map(|_| OutputSlot {
                    image: None,
                    dims: (0, 0),
                    srgb_view: None,
                })
                .collect(),
            device: device.clone(),
        })
    }

    /// Convert NV12 luma+chroma VkImages to an RGBA8 VkImage via compute shader.
    /// `ring_index` must match the frame-in-flight index to avoid overwriting
    /// an output image that the GPU is still reading.
    /// Returns the SRGB view for sampling in the fragment shader.
    pub fn convert(
        &mut self,
        gpu: &GpuContext,
        cmd: vk::CommandBuffer,
        luma_view: vk::ImageView,
        chroma_view: vk::ImageView,
        width: u32,
        height: u32,
        ring_index: usize,
    ) -> Result<vk::ImageView> {
        let idx = ring_index % OUTPUT_RING_SIZE;
        let slot = &mut self.output_slots[idx];

        let needs_recreate = match &slot.image {
            Some(_) => slot.dims != (width, height),
            None => true,
        };

        if needs_recreate {
            // Destroy old SRGB view before replacing image
            if let Some(old_view) = slot.srgb_view.take() {
                unsafe { self.device.destroy_image_view(old_view, None); }
            }

            // MUTABLE_FORMAT allows creating SRGB views from UNORM image
            let image = Image::new_with_flags(
                gpu,
                width,
                height,
                vk::Format::R8G8B8A8_UNORM,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::SAMPLED,
                MemoryLocation::GpuOnly,
                vk::ImageCreateFlags::MUTABLE_FORMAT,
            )?;

            // Create SRGB view for sampling (fragment shader expects sRGB decode)
            let srgb_view_info = vk::ImageViewCreateInfo::default()
                .image(image.raw())
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(vk::Format::R8G8B8A8_SRGB)
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            let srgb_view = unsafe { self.device.create_image_view(&srgb_view_info, None)? };

            slot.image = Some(image);
            slot.dims = (width, height);
            slot.srgb_view = Some(srgb_view);
        }

        let output = slot.image.as_ref().unwrap();

        let device = gpu.ash_device();
        let desc_set = self.descriptor_sets[idx];

        // Update descriptors
        let luma_info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(luma_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let chroma_info = vk::DescriptorImageInfo::default()
            .sampler(self.sampler)
            .image_view(chroma_view)
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

        let output_info = vk::DescriptorImageInfo::default()
            .image_view(output.view())
            .image_layout(vk::ImageLayout::GENERAL);

        let writes = [
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&luma_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(std::slice::from_ref(&chroma_info)),
            vk::WriteDescriptorSet::default()
                .dst_set(desc_set)
                .dst_binding(2)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(std::slice::from_ref(&output_info)),
        ];

        unsafe {
            device.update_descriptor_sets(&writes, &[]);
        }

        // Transition output image to GENERAL for compute write
        unsafe {
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(output.raw())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::empty())
                .dst_access_mask(vk::AccessFlags::SHADER_WRITE);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );

            // Bind and dispatch
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, self.pipeline);
            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                &[desc_set],
                &[],
            );

            let group_x = (width + 15) / 16;
            let group_y = (height + 15) / 16;
            device.cmd_dispatch(cmd, group_x, group_y, 1);

            // Transition output to SHADER_READ_ONLY for sampling
            let barrier = vk::ImageMemoryBarrier::default()
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
                .image(output.raw())
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                })
                .src_access_mask(vk::AccessFlags::SHADER_WRITE)
                .dst_access_mask(vk::AccessFlags::SHADER_READ);

            device.cmd_pipeline_barrier(
                cmd,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
        }

        Ok(self.output_slots[idx].srgb_view.unwrap())
    }

    pub fn destroy(&mut self) {
        unsafe {
            for slot in &mut self.output_slots {
                if let Some(v) = slot.srgb_view.take() {
                    self.device.destroy_image_view(v, None);
                }
                slot.image = None;
            }
            self.device.destroy_sampler(self.sampler, None);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.device
                .destroy_descriptor_pool(self.descriptor_pool, None);
            self.device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}
