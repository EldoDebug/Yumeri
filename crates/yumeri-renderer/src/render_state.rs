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
use crate::texture::store::TextureStore;
use crate::ui::renderer::UiRenderer;
use crate::ui::Scene;

pub struct WindowRenderState {
    surface: Surface,
    swapchain: Swapchain,
    frame_sync: FrameSynchronizer,
    renderer2d: Option<Renderer2D>,
    ui_renderer: Option<UiRenderer>,
    texture_store: Option<TextureStore>,
}

impl WindowRenderState {
    pub fn new(
        gpu: &GpuContext,
        display_handle: RawDisplayHandle,
        window_handle: RawWindowHandle,
        width: u32,
        height: u32,
        enable_2d: bool,
        enable_ui: bool,
    ) -> Result<Self> {
        let surface = Surface::new(gpu, display_handle, window_handle)?;
        let swapchain = Swapchain::new(gpu, &surface, width, height)?;
        let frame_sync = FrameSynchronizer::new(gpu)?;

        let needs_textures = enable_2d || enable_ui;
        let texture_store = if needs_textures {
            Some(TextureStore::new(gpu)?)
        } else {
            None
        };

        let tex_layout = texture_store.as_ref().map(|s| s.descriptor_set_layout());

        let renderer2d = if enable_2d {
            let mut r = Renderer2D::new();
            r.initialize_with_textures(gpu, swapchain.format().format, tex_layout.unwrap())?;
            Some(r)
        } else {
            None
        };

        let ui_renderer = if enable_ui {
            let mut r = UiRenderer::new();
            r.initialize(gpu, swapchain.format().format, tex_layout.unwrap())?;
            Some(r)
        } else {
            None
        };

        Ok(Self {
            surface,
            swapchain,
            frame_sync,
            renderer2d,
            ui_renderer,
            texture_store,
        })
    }

    pub fn render_frame(
        &mut self,
        gpu: &GpuContext,
        on_render2d: impl FnOnce(&mut RenderContext2D),
        ui_scene: Option<&mut Scene>,
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

        // Process pending async texture loads
        if let Some(store) = &mut self.texture_store {
            store.process_pending(gpu);
            store.flush_descriptors(frame_index);
        }

        if let (Some(r2d), Some(store)) = (&mut self.renderer2d, &mut self.texture_store) {
            let mut ctx = RenderContext2D {
                renderer: r2d,
                texture_store: store,
                gpu,
                surface_size: (extent.width, extent.height),
            };
            on_render2d(&mut ctx);
        }

        let mut builder = RenderGraphBuilder::new();
        let backbuffer = builder.import_backbuffer();

        // UI pass first (drawn underneath)
        if let (Some(ui_r), Some(scene), Some(store)) =
            (&mut self.ui_renderer, ui_scene, &self.texture_store)
        {
            ui_r.sync_and_register(scene, store, &mut builder, backbuffer, frame_index);
        }

        // 2D overlay pass (drawn on top)
        if let (Some(r2d), Some(store)) = (&mut self.renderer2d, &self.texture_store) {
            r2d.register_passes_with_textures(store, &mut builder, backbuffer, frame_index);
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
        Ok(())
    }

    pub fn destroy(&mut self, gpu: &GpuContext) {
        unsafe {
            let _ = gpu.ash_device().device_wait_idle();
        }
        if let Some(r2d) = &mut self.renderer2d {
            r2d.destroy(gpu);
        }
        if let Some(ui_r) = &mut self.ui_renderer {
            ui_r.destroy(gpu);
        }
        if let Some(store) = &mut self.texture_store {
            store.destroy();
        }
    }
}
