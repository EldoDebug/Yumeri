use wayland_server::protocol::wl_shm;

use crate::compositor::ShmBufferSpec;

pub fn shm_buffer_to_image(
    pool_data: &[u8],
    spec: &ShmBufferSpec,
) -> Option<yumeri_image::Image> {
    if spec.width <= 0 || spec.height <= 0 || spec.stride <= 0 || spec.offset < 0 {
        return None;
    }

    let width = spec.width as u32;
    let height = spec.height as u32;
    let stride = spec.stride as usize;
    let offset = spec.offset as usize;
    let bpp = match spec.format {
        wl_shm::Format::Argb8888 | wl_shm::Format::Xrgb8888 => 4,
        _ => return None,
    };

    let required = offset + (height as usize - 1) * stride + width as usize * bpp;
    if required > pool_data.len() {
        return None;
    }

    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    let opaque = spec.format == wl_shm::Format::Xrgb8888;

    for y in 0..height as usize {
        let row_start = offset + y * stride;
        let row = &pool_data[row_start..row_start + width as usize * bpp];
        for pixel in row.chunks_exact(4) {
            rgba.push(pixel[2]); // R
            rgba.push(pixel[1]); // G
            rgba.push(pixel[0]); // B
            rgba.push(if opaque { 255 } else { pixel[3] }); // A
        }
    }

    Some(yumeri_image::Image::from_raw(
        rgba,
        width,
        height,
        yumeri_image::PixelFormat::Rgba8,
    ))
}
