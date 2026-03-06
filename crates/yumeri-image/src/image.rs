use std::path::Path;

use crate::backend;
use crate::error::Result;
use crate::pixel_format::PixelFormat;

#[derive(Clone, Debug)]
pub struct Image {
    data: Vec<u8>,
    width: u32,
    height: u32,
    format: PixelFormat,
}

impl Image {
    pub fn from_raw(data: Vec<u8>, width: u32, height: u32, format: PixelFormat) -> Self {
        let expected = width as usize * height as usize * format.bytes_per_pixel();
        debug_assert_eq!(
            data.len(),
            expected,
            "buffer size mismatch: expected {}x{}x{} = {expected} bytes, got {}",
            width,
            height,
            format.bytes_per_pixel(),
            data.len(),
        );
        Self {
            data,
            width,
            height,
            format,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_with(path, PixelFormat::default())
    }

    pub fn load_with(path: impl AsRef<Path>, format: PixelFormat) -> Result<Self> {
        backend::decode_from_path(path.as_ref(), format)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        Self::decode_with(bytes, PixelFormat::default())
    }

    pub fn decode_with(bytes: &[u8], format: PixelFormat) -> Result<Self> {
        backend::decode_from_memory(bytes, format)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn into_data(self) -> Vec<u8> {
        self.data
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn format(&self) -> PixelFormat {
        self.format
    }

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    pub fn convert_to(&self, target: PixelFormat) -> Result<Self> {
        backend::convert(self, target)
    }
}
