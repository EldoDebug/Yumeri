#[derive(Clone, Copy, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Rect {
    pub position: [f32; 2], // center
    pub size: [f32; 2],     // half-extents
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedRect {
    pub position: [f32; 2],
    pub size: [f32; 2], // half-extents
    pub corner_radius: f32,
    pub color: Color,
}

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub position: [f32; 2], // center
    pub radius: f32,
    pub color: Color,
}

pub(crate) enum Shape {
    Rect(Rect),
    RoundedRect(RoundedRect),
    Circle(Circle),
}

// GPU-side instance data: 10 floats per instance
// [pos.x, pos.y, size.x, size.y, corner_radius, shape_type, r, g, b, a]
// shape_type: 0.0 = Rect, 1.0 = RoundedRect, 2.0 = Circle
pub(crate) const FLOATS_PER_INSTANCE: usize = 10;

impl Shape {
    pub(crate) fn to_instance_data(&self) -> [f32; FLOATS_PER_INSTANCE] {
        match self {
            Shape::Rect(r) => [
                r.position[0],
                r.position[1],
                r.size[0],
                r.size[1],
                0.0, // no corner radius
                0.0, // shape_type = Rect
                r.color.r,
                r.color.g,
                r.color.b,
                r.color.a,
            ],
            Shape::RoundedRect(r) => [
                r.position[0],
                r.position[1],
                r.size[0],
                r.size[1],
                r.corner_radius,
                1.0, // shape_type = RoundedRect
                r.color.r,
                r.color.g,
                r.color.b,
                r.color.a,
            ],
            Shape::Circle(c) => [
                c.position[0],
                c.position[1],
                c.radius,
                c.radius, // size = radius for circle
                0.0,
                2.0, // shape_type = Circle
                c.color.r,
                c.color.g,
                c.color.b,
                c.color.a,
            ],
        }
    }
}
