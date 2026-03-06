use std::path::PathBuf;

use yumeri_font::Font;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::renderer::renderer2d::{Circle, Rect, Renderer2D, RoundedRect};
use crate::text::{shape_and_cache_glyphs, TextStyle};
use crate::texture::glyph_cache::GlyphCache;
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

pub struct RenderContext2D<'a> {
    pub(crate) renderer: &'a mut Renderer2D,
    pub(crate) texture_store: &'a mut TextureStore,
    pub(crate) glyph_cache: &'a mut GlyphCache,
    pub(crate) gpu: &'a GpuContext,
    pub(crate) surface_size: (u32, u32),
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
        self.texture_store.load(self.gpu, path)
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
        let (layout_glyphs, atlas_id) = shape_and_cache_glyphs(font, text, style, self.glyph_cache);

        for lg in layout_glyphs {
            let texture = atlas_id.map(|id| crate::texture::Texture { id, uv_rect: lg.cached.uv });
            self.renderer.draw_rect(lg.to_rect(position, style.color, texture));
        }
    }
}
