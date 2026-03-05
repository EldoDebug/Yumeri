use std::path::PathBuf;

use super::scene::Scene;
use crate::error::{RendererError, Result};
use crate::gpu::GpuContext;
use crate::texture::store::TextureStore;
use crate::texture::TextureId;

pub struct UiContext<'a> {
    scene: &'a mut Scene,
    textures: Option<(&'a mut TextureStore, &'a GpuContext)>,
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
            surface_size,
        }
    }

    pub fn with_textures(
        scene: &'a mut Scene,
        texture_store: &'a mut TextureStore,
        gpu: &'a GpuContext,
        surface_size: (u32, u32),
    ) -> Self {
        Self {
            scene,
            textures: Some((texture_store, gpu)),
            surface_size,
        }
    }

    pub fn scene(&mut self) -> &mut Scene {
        self.scene
    }

    pub fn surface_size(&self) -> (u32, u32) {
        self.surface_size
    }

    pub fn create_texture(&mut self, image: &yumeri_image::Image) -> Result<TextureId> {
        let (store, gpu) = self
            .textures
            .as_mut()
            .ok_or_else(|| RendererError::Texture("texture store not available".into()))?;
        store.create(gpu, image)
    }

    pub fn load_texture(&mut self, path: impl Into<PathBuf>) -> Result<TextureId> {
        let (store, gpu) = self
            .textures
            .as_mut()
            .ok_or_else(|| RendererError::Texture("texture store not available".into()))?;
        Ok(store.load(gpu, path))
    }
}
