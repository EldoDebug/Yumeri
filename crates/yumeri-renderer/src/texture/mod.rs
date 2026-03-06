pub(crate) mod glyph_cache;
pub(crate) mod gpu_texture;
pub(crate) mod store;

use slotmap::new_key_type;

new_key_type! { pub struct TextureId; }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UvRect {
    pub u_min: f32,
    pub v_min: f32,
    pub u_max: f32,
    pub v_max: f32,
}

impl Default for UvRect {
    fn default() -> Self {
        Self {
            u_min: 0.0,
            v_min: 0.0,
            u_max: 1.0,
            v_max: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Texture {
    pub id: TextureId,
    pub uv_rect: UvRect,
}

impl Texture {
    pub fn new(id: TextureId) -> Self {
        Self {
            id,
            uv_rect: UvRect::default(),
        }
    }

    pub fn uv(mut self, u_min: f32, v_min: f32, u_max: f32, v_max: f32) -> Self {
        self.uv_rect = UvRect {
            u_min,
            v_min,
            u_max,
            v_max,
        };
        self
    }
}
