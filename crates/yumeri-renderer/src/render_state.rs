use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::context::RenderContext2D;
use crate::error::{RendererError, Result};
use crate::frame::FrameSynchronizer;
use crate::gpu::surface::Surface;
use crate::gpu::swapchain::Swapchain;
use crate::gpu::GpuContext;
use crate::graph::{CompiledGraph, GraphExecutor, RenderGraphBuilder};
use crate::renderer::renderer2d::Renderer2D;
use crate::renderer::Renderer;

pub struct WindowRenderState {
    surface: Surface,
    swapchain: Swapchain,
    frame_sync: FrameSynchronizer,
    renderer2d: Option<Renderer2D>,
}

impl WindowRenderState {
    pub fn new(
        gpu: &GpuContext,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
        enable_2d: bool,
    ) -> Result<Self> {
        let surface = Surface::new(gpu, display_handle, window_handle)?;
        let swapchain = Swapchain::new(gpu, &surface, width, height)?;
        let frame_sync = FrameSynchronizer::new(gpu)?;

        let renderer2d = if enable_2d {
            let mut r = Renderer2D::new();
            r.initialize(gpu, swapchain.format().format)?;
            Some(r)
        } else {
            None
        };

        Ok(Self {
            surface,
            swapchain,
            frame_sync,
            renderer2d,
        })
    }

    pub fn render_frame(
        &mut self,
        gpu: &GpuContext,
        on_render2d: impl FnOnce(&mut RenderContext2D),
    ) -> Result<()> {
        let extent = self.swapchain.extent();
        if extent.width == 0 || extent.height == 0 {
            return Ok(());
        }

        let frame = match self.frame_sync.begin_frame(gpu, &self.swapchain) {
            Ok(f) => f,
            Err(RendererError::Vulkan(vk::Result::ERROR_OUT_OF_DATE_KHR)) => {
                let extent = self.swapchain.extent();
                self.swapchain
                    .recreate(gpu, &self.surface, extent.width, extent.height)?;
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let frame_index = self.frame_sync.current_frame();

        if let Some(r2d) = &mut self.renderer2d {
            let mut ctx = RenderContext2D {
                renderer: r2d,
                surface_size: (extent.width, extent.height),
            };
            on_render2d(&mut ctx);
        }

        let mut builder = RenderGraphBuilder::new();
        let backbuffer = builder.import_backbuffer();

        if let Some(r2d) = &mut self.renderer2d {
            r2d.register_passes(&mut builder, backbuffer, frame_index);
        }

        let (passes, resources, bb) = builder.build();
        let mut compiled = CompiledGraph::compile(passes, resources, bb);

        let img_idx = frame.swapchain_image_index as usize;
        GraphExecutor::execute(
            gpu.ash_device(),
            frame.command_buffer,
            &mut compiled,
            self.swapchain.images()[img_idx],
            self.swapchain.image_views()[img_idx],
            extent,
            self.swapchain.format().format,
        );

        let needs_recreate = self.frame_sync.end_frame(gpu, &self.swapchain, &frame)?;
        if needs_recreate {
            let extent = self.swapchain.extent();
            self.swapchain
                .recreate(gpu, &self.surface, extent.width, extent.height)?;
        }
        Ok(())
    }

    pub fn on_resize(&mut self, gpu: &GpuContext, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.swapchain.recreate(gpu, &self.surface, width, height)?;
        if let Some(r2d) = &mut self.renderer2d {
            r2d.on_resize(gpu, width, height)?;
        }
        Ok(())
    }

    pub fn destroy(&mut self, gpu: &GpuContext) {
        unsafe {
            let _ = gpu.ash_device().device_wait_idle();
        }
        if let Some(r2d) = &mut self.renderer2d {
            r2d.destroy(gpu);
        }
    }
}
