use glam::{Mat4, Vec2, Vec3};
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy)]
pub struct ModelMatrix {
    base_width: f32,
    base_height: f32,
    scale_x: f32,
    scale_y: f32,
    translate_x: f32,
    translate_y: f32,
}

impl ModelMatrix {
    /// Create a matrix helper for a model with the given base size.
    pub fn new(base_width: f32, base_height: f32) -> Self {
        let mut m = Self {
            base_width,
            base_height,
            scale_x: 1.0,
            scale_y: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        };
        if base_height.abs() > f32::EPSILON {
            m.set_height(2.0);
        }
        m
    }

    /// Set the model width in view units (uniform scale).
    pub fn set_width(&mut self, w: f32) {
        if self.base_width.abs() <= f32::EPSILON {
            return;
        }
        let scale = w / self.base_width;
        self.scale_x = scale;
        self.scale_y = scale;
    }

    /// Set the model height in view units (uniform scale).
    pub fn set_height(&mut self, h: f32) {
        if self.base_height.abs() <= f32::EPSILON {
            return;
        }
        let scale = h / self.base_height;
        self.scale_x = scale;
        self.scale_y = scale;
    }

    /// Set the translation in view units.
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.translate_x = x;
        self.translate_y = y;
    }

    /// Center the model around the given point in view units.
    pub fn set_center_position(&mut self, x: f32, y: f32) {
        self.center_x(x);
        self.center_y(y);
    }

    /// Align the model's top edge to `y` in view units.
    pub fn top(&mut self, y: f32) {
        self.set_y(y);
    }

    /// Align the model's bottom edge to `y` in view units.
    pub fn bottom(&mut self, y: f32) {
        let h = self.base_height * self.scale_y;
        self.translate_y = y - h;
    }

    /// Align the model's left edge to `x` in view units.
    pub fn left(&mut self, x: f32) {
        self.set_x(x);
    }

    /// Align the model's right edge to `x` in view units.
    pub fn right(&mut self, x: f32) {
        let w = self.base_width * self.scale_x;
        self.translate_x = x - w;
    }

    /// Center the model on `x` in view units.
    pub fn center_x(&mut self, x: f32) {
        let w = self.base_width * self.scale_x;
        self.translate_x = x - (w / 2.0);
    }

    /// Center the model on `y` in view units.
    pub fn center_y(&mut self, y: f32) {
        let h = self.base_height * self.scale_y;
        self.translate_y = y - (h / 2.0);
    }

    /// Set the X translation in view units.
    pub fn set_x(&mut self, x: f32) {
        self.translate_x = x;
    }

    /// Set the Y translation in view units.
    pub fn set_y(&mut self, y: f32) {
        self.translate_y = y;
    }

    /// Apply layout keys (scale first, then position) from a `model3.json` layout map.
    pub fn setup_from_layout(&mut self, layout: &IndexMap<String, f32>) {
        for (key, &value) in layout {
            match key.as_str() {
                "width" => self.set_width(value),
                "height" => self.set_height(value),
                _ => {}
            }
        }

        for (key, &value) in layout {
            match key.as_str() {
                "x" => self.set_x(value),
                "y" => self.set_y(value),
                "center_x" => self.center_x(value),
                "center_y" => self.center_y(value),
                "top" => self.top(value),
                "bottom" => self.bottom(value),
                "left" => self.left(value),
                "right" => self.right(value),
                _ => {}
            }
        }
    }

    /// Build a `Mat4` representing the current scale and translation.
    pub fn mat4(&self) -> Mat4 {
        let s = Mat4::from_scale(Vec3::new(self.scale_x, self.scale_y, 1.0));
        let t = Mat4::from_translation(Vec3::new(self.translate_x, self.translate_y, 0.0));
        t * s
    }

    /// Invert-transform a view-space point back into model space.
    pub fn invert_transform_point(&self, p: Vec2) -> Vec2 {
        Vec2::new(self.invert_transform_x(p.x), self.invert_transform_y(p.y))
    }

    /// Invert-transform a view-space X coordinate back into model space.
    pub fn invert_transform_x(&self, x: f32) -> f32 {
        if self.scale_x.abs() <= f32::EPSILON {
            return 0.0;
        }
        (x - self.translate_x) / self.scale_x
    }

    /// Invert-transform a view-space Y coordinate back into model space.
    pub fn invert_transform_y(&self, y: f32) -> f32 {
        if self.scale_y.abs() <= f32::EPSILON {
            return 0.0;
        }
        (y - self.translate_y) / self.scale_y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_width_and_height_overwrite_scale_like_native() {
        let mut m = ModelMatrix::new(10.0, 20.0);
        m.set_width(5.0);
        assert!((m.scale_x - 0.5).abs() < 1e-6);
        assert!((m.scale_y - 0.5).abs() < 1e-6);

        m.set_height(2.0);
        assert!((m.scale_x - 0.1).abs() < 1e-6);
        assert!((m.scale_y - 0.1).abs() < 1e-6);
    }

    #[test]
    fn translate_setters_are_absolute_like_native() {
        let mut m = ModelMatrix::new(10.0, 20.0);
        m.set_x(1.0);
        m.set_y(2.0);
        assert!((m.translate_x - 1.0).abs() < 1e-6);
        assert!((m.translate_y - 2.0).abs() < 1e-6);

        m.set_position(3.0, 4.0);
        assert!((m.translate_x - 3.0).abs() < 1e-6);
        assert!((m.translate_y - 4.0).abs() < 1e-6);
    }

    #[test]
    fn layout_helpers_match_native_formulas() {
        let mut m = ModelMatrix::new(10.0, 20.0);
        assert!((m.scale_x - 0.1).abs() < 1e-6);
        assert!((m.scale_y - 0.1).abs() < 1e-6);

        m.bottom(1.0);
        let expected_h = 20.0 * 0.1;
        assert!((m.translate_y - (1.0 - expected_h)).abs() < 1e-6);

        m.center_x(2.0);
        let expected_w = 10.0 * 0.1;
        assert!((m.translate_x - (2.0 - expected_w / 2.0)).abs() < 1e-6);

        let p = Vec2::new(0.25, -0.5);
        let view = (m.mat4() * Vec3::new(p.x, p.y, 1.0).extend(1.0)).truncate();
        let inv = m.invert_transform_point(Vec2::new(view.x, view.y));
        assert!((inv.x - p.x).abs() < 1e-5);
        assert!((inv.y - p.y).abs() < 1e-5);
    }
}
