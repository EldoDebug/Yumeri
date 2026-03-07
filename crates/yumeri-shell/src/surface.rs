use yumeri_renderer::RenderContext2D;

use crate::anchor::Anchor;
use crate::layer::Layer;

slotmap::new_key_type! { pub struct LayerSurfaceId; }

pub struct LayerBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

pub trait LayerSurface {
    fn layer(&self) -> Layer;

    fn anchor(&self) -> Anchor {
        Anchor::FILL
    }

    fn exclusive_zone(&self) -> u32 {
        0
    }

    fn desired_size(&self) -> (u32, u32) {
        (0, 0)
    }

    fn render(&self, ctx: &mut RenderContext2D, bounds: LayerBounds);
}
