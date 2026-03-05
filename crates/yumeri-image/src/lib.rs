mod backend;
pub(crate) mod error;
pub(crate) mod image;
pub(crate) mod pixel_format;

pub use error::{ImageError, Result};
pub use pixel_format::PixelFormat;
pub use self::image::Image;

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_DATA_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test_data");

    fn test_path(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(TEST_DATA_DIR).join(name)
    }

    #[test]
    fn pixel_format_bytes_per_pixel() {
        assert_eq!(PixelFormat::Rgba8.bytes_per_pixel(), 4);
        assert_eq!(PixelFormat::Rgb8.bytes_per_pixel(), 3);
        assert_eq!(PixelFormat::Grayscale8.bytes_per_pixel(), 1);
        assert_eq!(PixelFormat::GrayscaleAlpha8.bytes_per_pixel(), 2);
    }

    #[test]
    fn pixel_format_default_is_rgba8() {
        assert_eq!(PixelFormat::default(), PixelFormat::Rgba8);
    }

    #[test]
    fn load_1x1_white_png() {
        let img = Image::load(test_path("1x1_white.png")).unwrap();
        assert_eq!(img.width(), 1);
        assert_eq!(img.height(), 1);
        assert_eq!(img.dimensions(), (1, 1));
        assert_eq!(img.format(), PixelFormat::Rgba8);
        assert_eq!(img.byte_len(), 4);
        assert_eq!(img.data(), &[255, 255, 255, 255]);
    }

    #[test]
    fn load_2x2_red_png() {
        let img = Image::load(test_path("2x2_red.png")).unwrap();
        assert_eq!(img.dimensions(), (2, 2));
        assert_eq!(img.format(), PixelFormat::Rgba8);
        assert_eq!(img.byte_len(), 16);
        // All 4 pixels should be red (255, 0, 0, 255)
        for pixel in img.data().chunks_exact(4) {
            assert_eq!(pixel, &[255, 0, 0, 255]);
        }
    }

    #[test]
    fn load_with_rgb8() {
        let img = Image::load_with(test_path("1x1_white.png"), PixelFormat::Rgb8).unwrap();
        assert_eq!(img.format(), PixelFormat::Rgb8);
        assert_eq!(img.byte_len(), 3);
        assert_eq!(img.data(), &[255, 255, 255]);
    }

    #[test]
    fn load_with_grayscale() {
        let img =
            Image::load_with(test_path("1x1_white.png"), PixelFormat::Grayscale8).unwrap();
        assert_eq!(img.format(), PixelFormat::Grayscale8);
        assert_eq!(img.byte_len(), 1);
        assert_eq!(img.data(), &[255]);
    }

    #[test]
    fn decode_from_memory() {
        let bytes = std::fs::read(test_path("1x1_white.png")).unwrap();
        let img = Image::decode(&bytes).unwrap();
        assert_eq!(img.dimensions(), (1, 1));
        assert_eq!(img.data(), &[255, 255, 255, 255]);
    }

    #[test]
    fn decode_with_format() {
        let bytes = std::fs::read(test_path("2x2_red.png")).unwrap();
        let img = Image::decode_with(&bytes, PixelFormat::Grayscale8).unwrap();
        assert_eq!(img.format(), PixelFormat::Grayscale8);
        assert_eq!(img.byte_len(), 4); // 2x2, 1 byte/pixel
    }

    #[test]
    fn convert_rgba8_to_rgb8() {
        let img = Image::load(test_path("1x1_white.png")).unwrap();
        let converted = img.convert_to(PixelFormat::Rgb8).unwrap();
        assert_eq!(converted.format(), PixelFormat::Rgb8);
        assert_eq!(converted.data(), &[255, 255, 255]);
    }

    #[test]
    fn convert_same_format_returns_clone() {
        let img = Image::load(test_path("1x1_white.png")).unwrap();
        let converted = img.convert_to(PixelFormat::Rgba8).unwrap();
        assert_eq!(converted.data(), img.data());
    }

    #[test]
    fn into_data_consumes_image() {
        let img = Image::load(test_path("1x1_white.png")).unwrap();
        let data = img.into_data();
        assert_eq!(data, vec![255, 255, 255, 255]);
    }

    #[test]
    fn load_nonexistent_file_returns_io_error() {
        let result = Image::load("nonexistent.png");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ImageError::Io { .. }));
    }

    #[test]
    fn decode_invalid_bytes_returns_decode_error() {
        let result = Image::decode(&[0, 1, 2, 3]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ImageError::Decode(_)));
    }
}
