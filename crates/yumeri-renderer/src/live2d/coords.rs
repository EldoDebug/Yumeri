use glam::{Mat4, Vec2, Vec3, Vec4};

pub fn compute_projection_fit(width: u32, height: u32) -> Mat4 {
    let w = width as f32;
    let h = height as f32;
    if w < h {
        Mat4::from_scale(Vec3::new(1.0, w / h, 1.0))
    } else {
        Mat4::from_scale(Vec3::new(h / w, 1.0, 1.0))
    }
}

pub fn pixel_to_ndc(x: f64, y: f64, width: u32, height: u32) -> Vec2 {
    let w = width.max(1) as f32;
    let h = height.max(1) as f32;
    let nx = (x as f32 / w) * 2.0 - 1.0;
    let ny = 1.0 - (y as f32 / h) * 2.0;
    Vec2::new(nx, ny)
}

pub fn pixel_to_view(x: f64, y: f64, width: u32, height: u32) -> Vec2 {
    let ndc = pixel_to_ndc(x, y, width, height);
    let projection = compute_projection_fit(width, height);
    let inv_proj = projection.inverse();
    let p = inv_proj * Vec4::new(ndc.x, ndc.y, 0.0, 1.0);
    Vec2::new(p.x / p.w, p.y / p.w)
}
