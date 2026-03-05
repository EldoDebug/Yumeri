use ash::vk;

use super::scene::{Scene, SyncResult};
use crate::error::Result;
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};
use crate::renderer::instance_pipeline::{InstancePipeline, MAX_INSTANCES};
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

pub(crate) struct UiRenderer {
    ip: Option<InstancePipeline>,
    buffer_generations: Vec<u64>,
}

impl UiRenderer {
    pub(crate) fn new() -> Self {
        Self {
            ip: None,
            buffer_generations: Vec::new(),
        }
    }

    pub(crate) fn initialize(
        &mut self,
        gpu: &GpuContext,
        color_format: vk::Format,
        texture_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<()> {
        let ip = InstancePipeline::new(gpu, color_format, texture_descriptor_set_layout)?;
        let frames = ip.instance_buffers.len();
        self.ip = Some(ip);
        self.buffer_generations = vec![0; frames];
        Ok(())
    }

    pub(crate) fn sync_and_register(
        &mut self,
        scene: &mut Scene,
        texture_store: &TextureStore,
        builder: &mut RenderGraphBuilder,
        backbuffer: ResourceId,
        frame_index: usize,
    ) {
        let resolve = |id: TextureId| texture_store.resolve(id);
        let sync_result = scene.sync(resolve);

        let instance_count = scene.render_list().instance_count().min(MAX_INSTANCES as u32);
        if instance_count == 0 {
            return;
        }

        self.upload_buffer(scene, &sync_result, frame_index);

        let ip = self.ip.as_ref().unwrap();
        let pipeline = ip.pipeline.pipeline;
        let pipeline_layout = ip.pipeline.pipeline_layout;
        let ssbo_descriptor_set = ip.descriptor_sets[frame_index];
        let texture_descriptor_set = texture_store.descriptor_set(frame_index);

        builder.add_pass("ui_render", move |pass| {
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
    }

    fn upload_buffer(&mut self, scene: &Scene, sync_result: &SyncResult, frame_index: usize) {
        let current_gen = scene.generation();
        let buffer_gen = self.buffer_generations[frame_index];

        let ip = self.ip.as_mut().unwrap();
        let Some(mapped) = ip.instance_buffers[frame_index].mapped_slice_mut() else {
            return;
        };

        let needs_full = match sync_result {
            SyncResult::Clean => buffer_gen < current_gen,
            SyncResult::Incremental(ranges) if buffer_gen + 1 == current_gen => {
                scene.render_list().write_ranges(mapped, ranges);
                false
            }
            _ => true,
        };

        if needs_full {
            scene.render_list().write_all(mapped);
        }

        self.buffer_generations[frame_index] = current_gen;
    }

    pub(crate) fn destroy(&mut self, gpu: &GpuContext) {
        if let Some(ip) = &mut self.ip {
            ip.destroy(gpu);
        }
    }
}
