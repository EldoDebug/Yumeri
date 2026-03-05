mod batch;
mod pipeline;
pub(crate) mod shapes;

use ash::vk;
use gpu_allocator::MemoryLocation;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};
use crate::resource::Buffer;

use super::{RenderPhase, Renderer};
use batch::DrawBatch;
use pipeline::Pipeline2D;
pub use shapes::{Circle, Color, Rect, RoundedRect};
pub(crate) use shapes::Shape;

use crate::frame::MAX_FRAMES_IN_FLIGHT;

const MAX_INSTANCES: usize = 4096;
const INITIAL_BUFFER_SIZE: u64 =
    (MAX_INSTANCES * shapes::FLOATS_PER_INSTANCE * size_of::<f32>()) as u64;

pub struct Renderer2D {
    pipeline: Option<Pipeline2D>,
    instance_buffers: Vec<Buffer>,
    descriptor_pool: vk::DescriptorPool,
    descriptor_sets: Vec<vk::DescriptorSet>,
    batch: DrawBatch,
}

impl Renderer2D {
    pub(crate) fn new() -> Self {
        Self {
            pipeline: None,
            instance_buffers: Vec::new(),
            descriptor_pool: vk::DescriptorPool::null(),
            descriptor_sets: Vec::new(),
            batch: DrawBatch::new(),
        }
    }

    pub fn draw_rect(&mut self, rect: Rect) {
        self.batch.push(Shape::Rect(rect));
    }

    pub fn draw_rounded_rect(&mut self, rounded_rect: RoundedRect) {
        self.batch.push(Shape::RoundedRect(rounded_rect));
    }

    pub fn draw_circle(&mut self, circle: Circle) {
        self.batch.push(Shape::Circle(circle));
    }
}

impl Renderer for Renderer2D {
    fn phase(&self) -> RenderPhase {
        RenderPhase::Overlay2D
    }

    fn initialize(&mut self, gpu: &GpuContext, color_format: vk::Format) -> Result<()> {
        let device = gpu.ash_device();

        let pipeline = Pipeline2D::new(device, color_format)?;

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

        // Create descriptor pool
        let pool_size = vk::DescriptorPoolSize::default()
            .ty(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(frames_in_flight as u32);

        let pool_info = vk::DescriptorPoolCreateInfo::default()
            .max_sets(frames_in_flight as u32)
            .pool_sizes(std::slice::from_ref(&pool_size));

        let descriptor_pool = unsafe { device.create_descriptor_pool(&pool_info, None)? };

        // Allocate descriptor sets
        let layouts: Vec<_> = (0..frames_in_flight)
            .map(|_| pipeline.descriptor_set_layout)
            .collect();
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&layouts);

        let descriptor_sets = unsafe { device.allocate_descriptor_sets(&alloc_info)? };

        // Update descriptor sets to point to instance buffers
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

        self.pipeline = Some(pipeline);
        self.instance_buffers = instance_buffers;
        self.descriptor_pool = descriptor_pool;
        self.descriptor_sets = descriptor_sets;

        Ok(())
    }

    fn register_passes(&mut self, builder: &mut RenderGraphBuilder, backbuffer: ResourceId, frame_index: usize) {
        if self.batch.is_empty() {
            return;
        }

        // Upload instance data to current frame's buffer
        let frame_idx = frame_index;
        if let Some(buffer) = self.instance_buffers.get_mut(frame_idx) {
            if let Some(mapped) = buffer.mapped_slice_mut() {
                self.batch.write_to_buffer(mapped);
            }
        }

        let instance_count = self.batch.instance_count().min(MAX_INSTANCES as u32);
        let pipeline = self.pipeline.as_ref().unwrap().pipeline;
        let pipeline_layout = self.pipeline.as_ref().unwrap().pipeline_layout;
        let descriptor_set = self.descriptor_sets[frame_idx];

        builder.add_pass("render_2d", move |pass| {
            pass.write(backbuffer);
            move |ctx: &mut crate::graph::RenderPassContext| {
                let device = ctx.device();
                let cmd = ctx.command_buffer();
                let extent = ctx.render_area();

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

                    // Push constants: viewport size
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
                        &[descriptor_set],
                        &[],
                    );

                    // Draw: 4 vertices (triangle strip quad), N instances
                    device.cmd_draw(cmd, 4, instance_count, 0, 0);
                }
            }
        });

        self.batch.clear();
    }

    fn on_resize(&mut self, _gpu: &GpuContext, _width: u32, _height: u32) -> Result<()> {
        // Pipeline uses dynamic viewport/scissor, nothing to recreate
        Ok(())
    }

    fn destroy(&mut self, gpu: &GpuContext) {
        let device = gpu.ash_device();
        if let Some(pipeline) = self.pipeline.take() {
            pipeline.destroy(device);
        }
        unsafe {
            if self.descriptor_pool != vk::DescriptorPool::null() {
                device.destroy_descriptor_pool(self.descriptor_pool, None);
            }
        }
        // Buffers dropped via their Drop impl
        self.instance_buffers.clear();
    }
}
