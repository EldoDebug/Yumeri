pub(crate) mod instance_pipeline;
pub(crate) mod renderer2d;

use ash::vk;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::graph::{RenderGraphBuilder, ResourceId};

#[allow(dead_code)]
pub enum RenderPhase {
    Scene3D,
    Overlay2D,
    UI,
    PostEffect,
}

pub(crate) trait Renderer {
    #[allow(dead_code)]
    fn phase(&self) -> RenderPhase;
    fn initialize(&mut self, gpu: &GpuContext, color_format: vk::Format) -> Result<()>;
    fn register_passes(&mut self, builder: &mut RenderGraphBuilder, backbuffer: ResourceId, frame_index: usize);
    fn on_resize(&mut self, gpu: &GpuContext, width: u32, height: u32) -> Result<()>;
    fn destroy(&mut self, gpu: &GpuContext);
}
