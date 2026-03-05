use crate::renderer::renderer2d::{Circle, Rect, Renderer2D, RoundedRect};

pub struct RenderContext2D<'a> {
    pub(crate) renderer: &'a mut Renderer2D,
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
}
