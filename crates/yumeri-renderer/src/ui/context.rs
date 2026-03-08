use std::path::PathBuf;

use yumeri_threading::ThreadPool;

use super::node::NodeId;
use super::scene::Scene;
use crate::error::{RendererError, Result};
use crate::gpu::GpuContext;
use crate::text::TextStyle;
use crate::texture::glyph_cache::GlyphCache;
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

pub struct UiContext<'a> {
    scene: &'a mut Scene,
    textures: Option<(&'a mut TextureStore, &'a GpuContext, &'a ThreadPool)>,
    glyph_cache: Option<&'a mut GlyphCache>,
    surface_size: (u32, u32),
}

impl<'a> UiContext<'a> {
    pub fn new(
        scene: &'a mut Scene,
        surface_size: (u32, u32),
    ) -> Self {
        Self {
            scene,
            textures: None,
            glyph_cache: None,
            surface_size,
        }
    }

    pub(crate) fn with_textures(
        scene: &'a mut Scene,
        texture_store: &'a mut TextureStore,
        glyph_cache: &'a mut GlyphCache,
        gpu: &'a GpuContext,
        pool: &'a ThreadPool,
        surface_size: (u32, u32),
    ) -> Self {
        Self {
            scene,
            textures: Some((texture_store, gpu, pool)),
            glyph_cache: Some(glyph_cache),
            surface_size,
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub fn scene_and_glyph_cache(&mut self) -> (&mut Scene, Option<&mut GlyphCache>) {
        (self.scene, self.glyph_cache.as_deref_mut())
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    pub fn create_texture(&mut self, image: &yumeri_image::Image) -> Result<TextureId> {
        let (store, gpu, _pool) = self
            .textures
            .as_mut()
            .ok_or_else(|| RendererError::Texture("texture store not available".into()))?;
        store.create(gpu, image)
    }

    pub fn load_texture(&mut self, path: impl Into<PathBuf>) -> Result<TextureId> {
        let (store, gpu, pool) = self
            .textures
            .as_mut()
            .ok_or_else(|| RendererError::Texture("texture store not available".into()))?;
        Ok(store.load(gpu, pool, path))
    }

    pub fn set_text(
        &mut self,
        node_id: NodeId,
        font: &mut yumeri_font::Font,
        text: &str,
        style: &TextStyle,
    ) -> Result<()> {
        let (store, gpu, _pool) = self
            .textures
            .as_mut()
            .ok_or_else(|| RendererError::Texture("texture store not available".into()))?;
        let gc = self
            .glyph_cache
            .as_mut()
            .ok_or_else(|| RendererError::Texture("glyph cache not available".into()))?;

        // Reborrow to satisfy the borrow checker
        let store: &mut TextureStore = *store;
        let gpu: &GpuContext = *gpu;
        let gc: &mut GlyphCache = *gc;

        self.scene.set_text(node_id, font, text, style, gc);

        // Flush atlas so the texture is available for rendering
        gc.flush(gpu, store)?;
        Ok(())
    }
}
