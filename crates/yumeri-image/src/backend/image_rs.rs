use std::path::Path;

use image::DynamicImage;

use crate::error::{ImageError, Result};
use crate::image::Image;
use crate::pixel_format::PixelFormat;

pub(crate) fn decode_from_path(path: &Path, format: PixelFormat) -> Result<Image> {
    let dynamic = image::open(path).map_err(|e| match e {
        image::ImageError::IoError(source) => ImageError::Io {
            path: path.to_path_buf(),
            source,
        },
        other => ImageError::Decode(other.to_string()),
    })?;
    Ok(dynamic_to_image(dynamic, format))
}

pub(crate) fn decode_from_memory(bytes: &[u8], format: PixelFormat) -> Result<Image> {
    let dynamic =
        image::load_from_memory(bytes).map_err(|e| ImageError::Decode(e.to_string()))?;
    Ok(dynamic_to_image(dynamic, format))
}

fn dynamic_to_image(dynamic: DynamicImage, format: PixelFormat) -> Image {
    let (width, height) = (dynamic.width(), dynamic.height());
    let data = convert_dynamic(dynamic, format);
    Image::from_raw(data, width, height, format)
}

pub(crate) fn convert(image: &Image, target: PixelFormat) -> Result<Image> {
    if image.format() == target {
        return Ok(image.clone());
    }

    let dynamic = to_dynamic(image)?;
    let data = convert_dynamic(dynamic, target);
    Ok(Image::from_raw(data, image.width(), image.height(), target))
}

fn convert_dynamic(dynamic: DynamicImage, format: PixelFormat) -> Vec<u8> {
    match format {
        PixelFormat::Rgba8 => dynamic.into_rgba8().into_raw(),
        PixelFormat::Rgb8 => dynamic.into_rgb8().into_raw(),
        PixelFormat::Grayscale8 => dynamic.into_luma8().into_raw(),
        PixelFormat::GrayscaleAlpha8 => dynamic.into_luma_alpha8().into_raw(),
    }
}

fn to_dynamic(image: &Image) -> Result<DynamicImage> {
    let (w, h) = image.dimensions();
    let format = image.format();
    let data = image.data().to_vec();
    let err = || ImageError::Decode(format!("invalid {format:?} buffer size"));

    match format {
        PixelFormat::Rgba8 => image::RgbaImage::from_raw(w, h, data)
            .map(DynamicImage::ImageRgba8)
            .ok_or_else(err),
        PixelFormat::Rgb8 => image::RgbImage::from_raw(w, h, data)
            .map(DynamicImage::ImageRgb8)
            .ok_or_else(err),
        PixelFormat::Grayscale8 => image::GrayImage::from_raw(w, h, data)
            .map(DynamicImage::ImageLuma8)
            .ok_or_else(err),
        PixelFormat::GrayscaleAlpha8 => image::GrayAlphaImage::from_raw(w, h, data)
            .map(DynamicImage::ImageLumaA8)
            .ok_or_else(err),
    }
}
