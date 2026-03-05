#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum ShapeType {
    None = -1,
    Rect = 0,
    RoundedRect = 1,
    Circle = 2,
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
pub(crate) const FLOATS_PER_INSTANCE: usize = 10;

pub(crate) fn pack_instance(
    position: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    shape_type: ShapeType,
    color: Color,
) -> [f32; FLOATS_PER_INSTANCE] {
    [
        position[0],
        position[1],
        size[0],
        size[1],
        corner_radius,
        shape_type as i32 as f32,
        color.r,
        color.g,
        color.b,
        color.a,
    ]
}

impl Shape {
    pub(crate) fn to_instance_data(&self) -> [f32; FLOATS_PER_INSTANCE] {
        match self {
            Shape::Rect(r) => {
                pack_instance(r.position, r.size, 0.0, ShapeType::Rect, r.color)
            }
            Shape::RoundedRect(r) => {
                pack_instance(r.position, r.size, r.corner_radius, ShapeType::RoundedRect, r.color)
            }
            Shape::Circle(c) => {
                pack_instance(c.position, [c.radius, c.radius], 0.0, ShapeType::Circle, c.color)
            }
        }
    }
}
