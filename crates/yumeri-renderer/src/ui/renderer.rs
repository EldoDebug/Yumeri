use ash::vk;

use super::scene::{Scene, SyncResult};
use crate::error::Result;
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};
use crate::renderer::instance_pipeline::{InstancePipeline, MAX_INSTANCES};

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

    pub(crate) fn initialize(&mut self, gpu: &GpuContext, color_format: vk::Format) -> Result<()> {
        let ip = InstancePipeline::new(gpu, color_format)?;
        let frames = ip.instance_buffers.len();
        self.ip = Some(ip);
        self.buffer_generations = vec![0; frames];
        Ok(())
    }

    pub(crate) fn sync_and_register(
        &mut self,
        scene: &mut Scene,
        builder: &mut RenderGraphBuilder,
        backbuffer: ResourceId,
        frame_index: usize,
    ) {
        let sync_result = scene.sync();

        let instance_count = scene.render_list().instance_count().min(MAX_INSTANCES as u32);
        if instance_count == 0 {
            return;
        }

        self.upload_buffer(scene, &sync_result, frame_index);

        let ip = self.ip.as_ref().unwrap();
        let pipeline = ip.pipeline.pipeline;
        let pipeline_layout = ip.pipeline.pipeline_layout;
        let descriptor_set = ip.descriptor_sets[frame_index];

        builder.add_pass("ui_render", move |pass| {
            pass.write(backbuffer);
            move |ctx: &mut crate::graph::RenderPassContext| {
                InstancePipeline::record_draw(
                    ctx.device(),
                    ctx.command_buffer(),
                    ctx.render_area(),
                    pipeline,
                    pipeline_layout,
                    descriptor_set,
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

        // With double-buffering, each buffer is typically 2 generations behind,
        // so incremental updates rarely apply. Use full rewrite when stale.
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
