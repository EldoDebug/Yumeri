#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PixelFormat {
    #[default]
    Rgba8,
    Rgb8,
    Grayscale8,
    GrayscaleAlpha8,
}

impl PixelFormat {
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8 => 4,
            Self::Rgb8 => 3,
            Self::Grayscale8 => 1,
            Self::GrayscaleAlpha8 => 2,
        }
    }
}
