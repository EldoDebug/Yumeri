use yumeri_renderer::{Rect, RenderContext2D};
use yumeri_shell::{Anchor, Layer, LayerBounds, LayerSurface};
use yumeri_types::Color;

pub struct SolidColorWallpaper {
    pub color: Color,
}

impl SolidColorWallpaper {
    pub fn new(color: Color) -> Self {
        Self { color }
    }
}

impl LayerSurface for SolidColorWallpaper {
    fn layer(&self) -> Layer {
        Layer::Background
    }

    fn anchor(&self) -> Anchor {
        Anchor::FILL
    }

    fn render(&self, ctx: &mut RenderContext2D, bounds: LayerBounds) {
        ctx.draw_rect(Rect {
            position: [
                bounds.x as f32 + bounds.width as f32 / 2.0,
                bounds.y as f32 + bounds.height as f32 / 2.0,
            ],
            size: [bounds.width as f32 / 2.0, bounds.height as f32 / 2.0],
            color: self.color,
            texture: None,
        });
    }
}
