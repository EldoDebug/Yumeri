use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::error::Result;
use crate::frame::MAX_FRAMES_IN_FLIGHT;
use crate::gpu::GpuContext;
use crate::resource::Buffer;

use super::renderer2d::pipeline::Pipeline2D;
use super::renderer2d::shapes::FLOATS_PER_INSTANCE;

pub(crate) const MAX_INSTANCES: usize = 4096;
pub(crate) const INITIAL_BUFFER_SIZE: u64 =
    (MAX_INSTANCES * FLOATS_PER_INSTANCE * size_of::<f32>()) as u64;

pub(crate) struct InstancePipeline {
    pub pipeline: Pipeline2D,
    pub instance_buffers: Vec<Buffer>,
    pub descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
}

impl InstancePipeline {
    pub fn new(
        gpu: &GpuContext,
        color_format: vk::Format,
        texture_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<Self> {
        let device = gpu.ash_device();

        let pipeline = Pipeline2D::new(device, color_format, texture_descriptor_set_layout)?;

        let frames_in_flight = MAX_FRAMES_IN_FLIGHT;
        let mut instance_buffers = Vec::with_capacity(frames_in_flight);
        for _ in 0..frames_in_flight {
            let buffer = Buffer::new(
                gpu,
                INITIAL_BUFFER_SIZE,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                MemoryLocation::CpuToGpu,
            )?;
            instance_buffers.push(buffer);
        }

        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(frames_in_flight as u32);

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(frames_in_flight as u32)
            .pool_sizes(std::slice::from_ref(&pool_size));

        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        let layouts: Vec<_> = (0..frames_in_flight)
            .map(|_| pipeline.ssbo_descriptor_set_layout)
            .collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        for (i, &set) in descriptor_sets.iter().enumerate() {
            let buffer_info = vk::DescriptorBufferInfo::default()
                .buffer(instance_buffers[i].raw())
                .offset(0)
                .range(vk::WHOLE_SIZE);

            let write = vk::WriteDescriptorSet::default()
                .dst_set(set)
                .dst_binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(std::slice::from_ref(&buffer_info));

            unsafe { device.update_descriptor_sets(&[write], &[]) };
        }

        Ok(Self {
            pipeline,
            instance_buffers,
            descriptor_pool,
            descriptor_sets,
        })
    }

    pub fn record_draw(
        device: &ash::Device,
        cmd: vk::CommandBuffer,
        extent: vk::Extent2D,
        pipeline: vk::Pipeline,
        pipeline_layout: vk::PipelineLayout,
        ssbo_descriptor_set: vk::DescriptorSet,
        texture_descriptor_set: vk::DescriptorSet,
        instance_count: u32,
    ) {
        unsafe {
            device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::GRAPHICS, pipeline);

            let viewport = vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: extent.width as f32,
                height: extent.height as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };
            device.cmd_set_viewport(cmd, 0, &[viewport]);

            let scissor = vk::Rect2D {
                offset: vk::Offset2D { x: 0, y: 0 },
                extent,
            };
            device.cmd_set_scissor(cmd, 0, &[scissor]);

            let viewport_size = [extent.width as f32, extent.height as f32];
            let push_data = bytemuck::cast_slice::<f32, u8>(&viewport_size);
            device.cmd_push_constants(
                cmd,
                pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                push_data,
            );

            device.cmd_bind_descriptor_sets(
                cmd,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline_layout,
                0,
                &[ssbo_descriptor_set, texture_descriptor_set],
                &[],
            );

            device.cmd_draw(cmd, 4, instance_count, 0, 0);
        }
    }

    pub fn destroy(&mut self, gpu: &GpuContext) {
        let device = gpu.ash_device();
        self.pipeline.destroy(device);
        unsafe {
            if self.descriptor_pool != vk::DescriptorPool::null() {
                device.destroy_descriptor_pool(self.descriptor_pool, None);
            }
        }
        self.instance_buffers.clear();
    }
}
