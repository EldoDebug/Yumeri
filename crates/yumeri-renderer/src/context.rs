use std::path::PathBuf;

use crate::error::Result;
use crate::gpu::GpuContext;
use crate::renderer::renderer2d::{Circle, Rect, Renderer2D, RoundedRect};
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

pub struct RenderContext2D<'a> {
    pub(crate) renderer: &'a mut Renderer2D,
    pub(crate) texture_store: &'a mut TextureStore,
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
}
