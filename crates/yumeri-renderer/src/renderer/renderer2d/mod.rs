mod batch;
pub(crate) mod shapes;

use ash::vk;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

use super::instance_pipeline::{InstancePipeline, MAX_INSTANCES};
use batch::DrawBatch;
pub use shapes::{Circle, Color, Rect, RoundedRect};
pub(crate) use shapes::Shape;

pub struct Renderer2D {
    ip: Option<InstancePipeline>,
    batch: DrawBatch,
}

impl Renderer2D {
    pub(crate) fn new() -> Self {
        Self {
            ip: None,
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

    pub(crate) fn initialize_with_textures(
        &mut self,
        gpu: &GpuContext,
        color_format: vk::Format,
        texture_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<()> {
        self.ip = Some(InstancePipeline::new(
            gpu,
            color_format,
            texture_descriptor_set_layout,
        )?);
        Ok(())
    }

    pub(crate) fn register_passes_with_textures(
        &mut self,
        texture_store: &TextureStore,
        builder: &mut RenderGraphBuilder,
        backbuffer: ResourceId,
        frame_index: usize,
    ) {
        if self.batch.is_empty() {
            return;
        }

        let ip = self.ip.as_mut().unwrap();
        let resolve = |id: TextureId| texture_store.resolve(id);

        if let Some(buffer) = ip.instance_buffers.get_mut(frame_index)
            && let Some(mapped) = buffer.mapped_slice_mut()
        {
            self.batch.write_to_buffer(mapped, resolve);
        }

        let instance_count = self.batch.instance_count().min(MAX_INSTANCES as u32);
        let pipeline = ip.pipeline.pipeline;
        let pipeline_layout = ip.pipeline.pipeline_layout;
        let ssbo_descriptor_set = ip.descriptor_sets[frame_index];
        let texture_descriptor_set = texture_store.descriptor_set(frame_index);

        builder.add_pass("render_2d", move |pass| {
            pass.write(backbuffer);
            move |ctx: &mut crate::graph::RenderPassContext| {
                InstancePipeline::record_draw(
                    ctx.device(),
                    ctx.command_buffer(),
                    ctx.render_area(),
                    pipeline,
                    pipeline_layout,
                    ssbo_descriptor_set,
                    texture_descriptor_set,
                    instance_count,
                );
            }
        });

        self.batch.clear();
    }

    pub(crate) fn destroy(&mut self, gpu: &GpuContext) {
        if let Some(ip) = &mut self.ip {
            ip.destroy(gpu);
        }
    }
}
