use ash::vk;
use raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use yumeri_threading::ThreadPool;

use crate::context::RenderContext2D;
use crate::error::{RendererError, Result};
use crate::frame::FrameSynchronizer;
use crate::gpu::surface::Surface;
use crate::gpu::swapchain::{Swapchain, SwapchainConfig};
use crate::gpu::GpuContext;
use crate::graph::{CompiledGraph, GraphExecutor, RenderGraphBuilder, ResourceId};
use crate::postfx::{PostEffect, PostEffectChain};
use crate::renderer::renderer2d::Renderer2D;
use crate::texture::glyph_cache::GlyphCache;
use crate::texture::store::TextureStore;
use crate::ui::renderer::UiRenderer;
use crate::ui::Scene;
use crate::video::VideoTexture;

pub struct WindowRenderState {
    surface: Surface,
    swapchain: Swapchain,
    swapchain_config: SwapchainConfig,
    frame_sync: FrameSynchronizer,
    renderer2d: Option<Renderer2D>,
    ui_renderer: Option<UiRenderer>,
    texture_store: Option<TextureStore>,
    glyph_cache: Option<GlyphCache>,
    video_textures: Vec<VideoTexture>,
    postfx_chain: Option<PostEffectChain>,
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
        swapchain_config: SwapchainConfig,
    ) -> Result<Self> {
        let surface = Surface::new(gpu, display_handle, window_handle)?;
        let swapchain = Swapchain::new(gpu, &surface, width, height, &swapchain_config)?;
        let frame_sync = FrameSynchronizer::new(gpu)?;

        let needs_textures = enable_2d || enable_ui;
        let texture_store = if needs_textures {
            Some(TextureStore::new(gpu)?)
        } else {
            None
        };

        let glyph_cache = if needs_textures {
            Some(GlyphCache::new())
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
            swapchain_config,
            frame_sync,
            renderer2d,
            ui_renderer,
            texture_store,
            glyph_cache,
            video_textures: Vec::new(),
            postfx_chain: None,
        })
    }

    pub fn render_frame(
        &mut self,
        gpu: &GpuContext,
        pool: &ThreadPool,
        on_render2d: impl FnOnce(&mut RenderContext2D),
        on_custom: Option<&mut dyn FnMut(&mut RenderGraphBuilder, ResourceId)>,
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
                    .recreate(gpu, &self.surface, extent.width, extent.height, &self.swapchain_config)?;
                return Ok(());
            }
            Err(e) => return Err(e),
        };

        let frame_index = self.frame_sync.current_frame();

        // Process pending async texture loads
        if let Some(store) = &mut self.texture_store {
            store.process_pending(gpu);
        }

        if let (Some(r2d), Some(store), Some(gc)) = (
            &mut self.renderer2d,
            &mut self.texture_store,
            &mut self.glyph_cache,
        ) {
            let mut ctx = RenderContext2D {
                renderer: r2d,
                texture_store: store,
                glyph_cache: gc,
                gpu,
                pool,
                surface_size: (extent.width, extent.height),
                video_textures: &mut self.video_textures,
                frame_index,
            };
            on_render2d(&mut ctx);
        }

        // Record streaming video uploads on the frame command buffer
        if let Some(store) = &mut self.texture_store {
            for vt in &mut self.video_textures {
                vt.record_upload(frame.command_buffer, gpu, store, frame_index);
            }
        }

        // Flush glyph atlas after draw_text calls, before descriptor flush
        if let (Some(gc), Some(store)) = (&mut self.glyph_cache, &mut self.texture_store) {
            if let Err(e) = gc.flush(gpu, store) {
                log::error!("Failed to flush glyph cache: {e}");
            }
        }

        if let Some(store) = &mut self.texture_store {
            store.flush_descriptors(frame_index);
        }

        let mut builder = RenderGraphBuilder::new();
        let backbuffer = builder.import_backbuffer();

        // UI pass first (drawn underneath)
        if let (Some(ui_r), Some(scene), Some(store)) =
            (&mut self.ui_renderer, ui_scene, &self.texture_store)
        {
            ui_r.sync_and_register(scene, store, &mut builder, backbuffer, frame_index);
        }

        // Custom pass (e.g. Live2D, drawn between UI and 2D overlay)
        if let Some(on_custom) = on_custom {
            on_custom(&mut builder, backbuffer);
        }

        // 2D overlay pass (drawn on top)
        if let (Some(r2d), Some(store)) = (&mut self.renderer2d, &self.texture_store) {
            r2d.register_passes_with_textures(store, &mut builder, backbuffer, frame_index);
        }

        let (passes, resources, bb) = builder.build();
        let mut compiled = CompiledGraph::compile(passes, resources, bb);

        let clear_alpha = if self.swapchain_config.transparent { 0.0 } else { 1.0 };
        let img_idx = frame.swapchain_image_index as usize;
        let has_postfx = self
            .postfx_chain
            .as_ref()
            .is_some_and(|c| !c.is_empty());

        GraphExecutor::execute(
            gpu.ash_device(),
            frame.command_buffer,
            &mut compiled,
            self.swapchain.images()[img_idx],
            self.swapchain.image_views()[img_idx],
            extent,
            [0.0, 0.0, 0.0, clear_alpha],
            has_postfx,
        );

        if let Some(chain) = &mut self.postfx_chain {
            chain.apply(
                gpu,
                frame.command_buffer,
                self.swapchain.images()[img_idx],
                extent,
                frame_index,
            )?;
        }

        let needs_recreate = self.frame_sync.end_frame(gpu, &self.swapchain, &frame)?;
        if needs_recreate {
            let extent = self.swapchain.extent();
            self.swapchain
                .recreate(gpu, &self.surface, extent.width, extent.height, &self.swapchain_config)?;
        }
        Ok(())
    }

    pub fn setup_ui_context<'a>(
        &'a mut self,
        scene: &'a mut Scene,
        gpu: &'a GpuContext,
        pool: &'a ThreadPool,
        surface_size: (u32, u32),
    ) -> crate::ui::UiContext<'a> {
        match (&mut self.texture_store, &mut self.glyph_cache) {
            (Some(store), Some(gc)) => {
                crate::ui::UiContext::with_textures(scene, store, gc, gpu, pool, surface_size)
            }
            _ => crate::ui::UiContext::new(scene, surface_size),
        }
    }

    pub fn swapchain_format(&self) -> vk::Format {
        self.swapchain.format().format
    }

    pub fn swapchain_extent(&self) -> (u32, u32) {
        let e = self.swapchain.extent();
        (e.width, e.height)
    }

    pub fn glyph_cache_mut(&mut self) -> Option<&mut GlyphCache> {
        self.glyph_cache.as_mut()
    }

    pub fn on_resize(&mut self, gpu: &GpuContext, width: u32, height: u32) -> Result<()> {
        if width == 0 || height == 0 {
            return Ok(());
        }
        self.swapchain.recreate(gpu, &self.surface, width, height, &self.swapchain_config)?;
        Ok(())
    }

    fn ensure_postfx_chain(&mut self, gpu: &GpuContext) -> Result<&mut PostEffectChain> {
        if self.postfx_chain.is_none() {
            self.postfx_chain = Some(PostEffectChain::new(gpu)?);
        }
        Ok(self.postfx_chain.as_mut().unwrap())
    }

    pub fn add_post_effect(
        &mut self,
        gpu: &GpuContext,
        effect: Box<dyn PostEffect>,
    ) -> Result<()> {
        self.ensure_postfx_chain(gpu)?.add(gpu, effect)
    }

    pub fn remove_post_effect(&mut self, name: &str) {
        if let Some(chain) = &mut self.postfx_chain {
            chain.remove(name);
        }
    }

    pub fn set_post_effect_mask(&mut self, mask_view: vk::ImageView) {
        if let Some(chain) = &mut self.postfx_chain {
            chain.set_mask(mask_view);
        }
    }

    pub fn set_post_effect_mask_from_data(
        &mut self,
        gpu: &GpuContext,
        width: u32,
        height: u32,
        data: &[u8],
    ) -> Result<()> {
        self.ensure_postfx_chain(gpu)?.set_mask_from_data(gpu, width, height, data)
    }

    pub fn clear_post_effect_mask(&mut self) {
        if let Some(chain) = &mut self.postfx_chain {
            chain.clear_mask();
        }
    }

    pub fn post_effect_chain(&self) -> Option<&PostEffectChain> {
        self.postfx_chain.as_ref()
    }

    pub fn post_effect_chain_mut(&mut self) -> Option<&mut PostEffectChain> {
        self.postfx_chain.as_mut()
    }

    pub fn destroy(&mut self, gpu: &GpuContext) {
        unsafe {
            let _ = gpu.ash_device().device_wait_idle();
        }
        if let Some(chain) = &mut self.postfx_chain {
            chain.destroy(gpu);
        }
        if let Some(r2d) = &mut self.renderer2d {
            r2d.destroy(gpu);
        }
        if let Some(ui_r) = &mut self.ui_renderer {
            ui_r.destroy(gpu);
        }
        if let Some(store) = &mut self.texture_store {
            for vt in &mut self.video_textures {
                vt.destroy(store);
            }
            self.video_textures.clear();
        }
        // Clear glyph cache before destroying texture store
        if let Some(gc) = &mut self.glyph_cache {
            gc.clear();
        }
        if let Some(store) = &mut self.texture_store {
            store.destroy();
        }
    }
}
