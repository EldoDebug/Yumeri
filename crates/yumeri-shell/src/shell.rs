use slotmap::SlotMap;
use yumeri_renderer::RenderContext2D;
use yumeri_types::ReservedRegions;

use crate::layer::Layer;
use crate::surface::{LayerBounds, LayerSurface, LayerSurfaceId};

pub struct LayerShell {
    surfaces: SlotMap<LayerSurfaceId, Box<dyn LayerSurface>>,
}

impl LayerShell {
    pub fn new() -> Self {
        Self {
            surfaces: SlotMap::with_key(),
        }
    }

    pub fn add(&mut self, surface: Box<dyn LayerSurface>) -> LayerSurfaceId {
        self.surfaces.insert(surface)
    }

    pub fn remove(&mut self, id: LayerSurfaceId) {
        self.surfaces.remove(id);
    }

    pub fn reserved_regions(&self, output_size: (u32, u32)) -> ReservedRegions {
        let mut regions = ReservedRegions::default();

        for (_, surface) in &self.surfaces {
            let zone = surface.exclusive_zone();
            if zone == 0 {
                continue;
            }
            let anchor = surface.anchor();
            let (desired_w, desired_h) = surface.desired_size();

            if anchor.top && !anchor.bottom {
                let h = if desired_h > 0 { desired_h } else { zone };
                regions.top += h;
            } else if anchor.bottom && !anchor.top {
                let h = if desired_h > 0 { desired_h } else { zone };
                regions.bottom += h;
            } else if anchor.left && !anchor.right {
                let w = if desired_w > 0 { desired_w } else { zone };
                regions.left += w;
            } else if anchor.right && !anchor.left {
                let w = if desired_w > 0 { desired_w } else { zone };
                regions.right += w;
            }
        }

        let _ = output_size; // available for future use
        regions
    }

    pub fn render_below_windows(&self, ctx: &mut RenderContext2D, output_size: (u32, u32)) {
        self.render_layers(ctx, output_size, |layer| {
            layer <= Layer::Bottom
        });
    }

    pub fn render_above_windows(&self, ctx: &mut RenderContext2D, output_size: (u32, u32)) {
        self.render_layers(ctx, output_size, |layer| {
            layer >= Layer::Top
        });
    }

    fn render_layers(
        &self,
        ctx: &mut RenderContext2D,
        output_size: (u32, u32),
        filter: impl Fn(Layer) -> bool,
    ) {
        let mut sorted: Vec<_> = self
            .surfaces
            .iter()
            .filter(|(_, s)| filter(s.layer()))
            .collect();
        sorted.sort_by_key(|(_, s)| s.layer());

        for (_, surface) in sorted {
            let bounds = compute_bounds(surface.as_ref(), output_size);
            surface.render(ctx, bounds);
        }
    }
}

impl Default for LayerShell {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_bounds(surface: &dyn LayerSurface, output_size: (u32, u32)) -> LayerBounds {
    let anchor = surface.anchor();
    let (desired_w, desired_h) = surface.desired_size();
    let (ow, oh) = output_size;

    let w = if desired_w > 0 { desired_w } else { ow };
    let (x, width) = match (anchor.left, anchor.right) {
        (true, _) => (0, w),
        (false, true) => (ow as i32 - w as i32, w),
        (false, false) => ((ow as i32 - w as i32) / 2, w),
    };

    let h = if desired_h > 0 { desired_h } else { oh };
    let (y, height) = match (anchor.top, anchor.bottom) {
        (true, _) => (0, h),
        (false, true) => (oh as i32 - h as i32, h),
        (false, false) => ((oh as i32 - h as i32) / 2, h),
    };

    LayerBounds {
        x,
        y,
        width,
        height,
    }
}
