#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VideoPixelFormat {
    #[default]
    Rgba8,
    Nv12,
    Yuv420P,
}
