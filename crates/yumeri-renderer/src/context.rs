use std::path::PathBuf;

use yumeri_font::Font;
use yumeri_threading::ThreadPool;
use yumeri_video::VideoHandle;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::renderer::renderer2d::{Circle, Rect, Renderer2D, RoundedRect};
use crate::text::{shape_and_cache_glyphs, TextStyle};
use crate::texture::glyph_cache::GlyphCache;
use crate::texture::store::TextureStore;
use crate::texture::TextureId;
use crate::video::VideoTexture;

pub struct RenderContext2D<'a> {
    pub(crate) renderer: &'a mut Renderer2D,
    pub(crate) texture_store: &'a mut TextureStore,
    pub(crate) glyph_cache: &'a mut GlyphCache,
    pub(crate) gpu: &'a GpuContext,
    pub(crate) pool: &'a ThreadPool,
    pub(crate) surface_size: (u32, u32),
    pub(crate) video_textures: &'a mut Vec<VideoTexture>,
    pub(crate) frame_index: usize,
}

impl<'a> RenderContext2D<'a> {
    pub fn draw_rect(&mut self, rect: Rect) {
        self.renderer.draw_rect(rect);
    }

    pub fn draw_rounded_rect(&mut self, rr: RoundedRect) {
        self.renderer.draw_rounded_rect(rr);
    }

    pub fn draw_circle(&mut self, circle: Circle) {
        self.renderer.draw_circle(circle);
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    pub fn create_texture(&mut self, image: &yumeri_image::Image) -> Result<TextureId> {
        self.texture_store.create(self.gpu, image)
    }

    pub fn load_texture(&mut self, path: impl Into<PathBuf>) -> TextureId {
        self.texture_store.load(self.gpu, self.pool, path)
    }

    pub fn thread_pool(&self) -> &ThreadPool {
        self.pool
    }

    pub fn remove_texture(&mut self, id: TextureId) {
        self.texture_store.remove(id);
    }

    pub fn draw_text(
        &mut self,
        font: &mut Font,
        text: &str,
        position: [f32; 2],
        style: &TextStyle,
    ) {
        let (layout_glyphs, atlas_id, _) = shape_and_cache_glyphs(font, text, style, self.glyph_cache);

        for lg in layout_glyphs {
            let texture = atlas_id.map(|id| crate::texture::Texture { id, uv_rect: lg.cached.uv });
            self.renderer.draw_rect(lg.to_rect(position, style.color, texture));
        }
    }

    /// Get `VulkanDeviceInfo` for Vulkan hardware-accelerated video decoding.
    pub fn vulkan_device_info(&self) -> yumeri_video::VulkanDeviceInfo {
        self.gpu.vulkan_device_info()
    }

    /// Create a video texture that automatically updates from a VideoHandle.
    pub fn create_video_texture(&mut self, handle: VideoHandle) -> Result<TextureId> {
        let vt = VideoTexture::new(self.gpu, self.texture_store, handle)?;
        let id = vt.texture_id();
        self.video_textures.push(vt);
        Ok(id)
    }

    /// Drain decoded video frames and stage them for GPU upload.
    /// Call this once per frame before drawing. The actual GPU copy is
    /// recorded automatically on the render command buffer.
    pub fn update_video_textures(&mut self) {
        for vt in self.video_textures.iter_mut() {
            vt.update(self.frame_index);
        }
    }
}
