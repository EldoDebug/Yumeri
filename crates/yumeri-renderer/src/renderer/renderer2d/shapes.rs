use crate::texture::{Texture, TextureId};
pub use yumeri_types::{Color, ShapeType};

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

// GPU-side instance data: 20 floats per instance (80 bytes)
// [pos.x, pos.y, size.x, size.y, corner_radius, shape_type, r, g, b, a,
//  texture_index, uv_min.x, uv_min.y, uv_max.x, uv_max.y,
//  cos_r, sin_r, scale_x, scale_y, _padding]
pub(crate) const FLOATS_PER_INSTANCE: usize = 20;
pub(crate) const NO_TEXTURE_INDEX: f32 = -1.0;

pub(crate) fn pack_instance(
    position: [f32; 2],
    size: [f32; 2],
    corner_radius: f32,
    shape_type: ShapeType,
    color: Color,
    texture: Option<Texture>,
    cos_r: f32,
    sin_r: f32,
    scale: [f32; 2],
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
        cos_r,
        sin_r,
        scale[0],
        scale[1],
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
                1.0, 0.0, [1.0, 1.0],
                resolve,
            ),
            Shape::RoundedRect(r) => pack_instance(
                r.position,
                r.size,
                r.corner_radius,
                ShapeType::RoundedRect,
                r.color,
                r.texture,
                1.0, 0.0, [1.0, 1.0],
                resolve,
            ),
            Shape::Circle(c) => pack_instance(
                c.position,
                [c.radius, c.radius],
                0.0,
                ShapeType::Circle,
                c.color,
                c.texture,
                1.0, 0.0, [1.0, 1.0],
                resolve,
            ),
        }
    }
}
