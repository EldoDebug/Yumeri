use crate::texture::{Texture, TextureId};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

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
    pub texture: Option<Texture>,
}

#[derive(Clone, Copy, Debug)]
pub struct RoundedRect {
    pub position: [f32; 2],
    pub size: [f32; 2], // half-extents
    pub corner_radius: f32,
    pub color: Color,
    pub texture: Option<Texture>,
}

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub position: [f32; 2], // center
    pub radius: f32,
    pub color: Color,
    pub texture: Option<Texture>,
}

pub(crate) enum Shape {
    Rect(Rect),
    RoundedRect(RoundedRect),
    Circle(Circle),
}

// GPU-side instance data: 16 floats per instance (64 bytes)
// [pos.x, pos.y, size.x, size.y, corner_radius, shape_type, r, g, b, a,
//  texture_index, uv_min.x, uv_min.y, uv_max.x, uv_max.y, _padding]
pub(crate) const FLOATS_PER_INSTANCE: usize = 16;
pub(crate) const NO_TEXTURE_INDEX: f32 = -1.0;

pub(crate) fn pack_instance(
    position: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    shape_type: ShapeType,
    color: Color,
    texture: Option<Texture>,
    resolve: impl Fn(TextureId) -> u32,
) -> [f32; FLOATS_PER_INSTANCE] {
    let (tex_index, uv_min_x, uv_min_y, uv_max_x, uv_max_y) = match texture {
        Some(t) => {
            let idx = resolve(t.id) as f32;
            (
                idx,
                t.uv_rect.u_min,
                t.uv_rect.v_min,
                t.uv_rect.u_max,
                t.uv_rect.v_max,
            )
        }
        None => (NO_TEXTURE_INDEX, 0.0, 0.0, 1.0, 1.0),
    };

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
        tex_index,
        uv_min_x,
        uv_min_y,
        uv_max_x,
        uv_max_y,
        0.0, // padding
    ]
}

impl Shape {
    pub(crate) fn to_instance_data(
        &self,
        resolve: impl Fn(TextureId) -> u32,
    ) -> [f32; FLOATS_PER_INSTANCE] {
        match self {
            Shape::Rect(r) => pack_instance(
                r.position,
                r.size,
                0.0,
                ShapeType::Rect,
                r.color,
                r.texture,
                resolve,
            ),
            Shape::RoundedRect(r) => pack_instance(
                r.position,
                r.size,
                r.corner_radius,
                ShapeType::RoundedRect,
                r.color,
                r.texture,
                resolve,
            ),
            Shape::Circle(c) => pack_instance(
                c.position,
                [c.radius, c.radius],
                0.0,
                ShapeType::Circle,
                c.color,
                c.texture,
                resolve,
            ),
        }
    }
}
