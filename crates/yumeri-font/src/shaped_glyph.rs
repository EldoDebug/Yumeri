use cosmic_text::CacheKey;

#[derive(Clone, Debug)]
pub struct ShapedGlyph {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub font_size: f32,
    pub line_index: usize,
    pub color: Option<[u8; 4]>,
    pub(crate) cache_key: GlyphCacheKey,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct GlyphCacheKey(pub(crate) CacheKey);

impl ShapedGlyph {
    pub fn cache_key(&self) -> GlyphCacheKey {
        self.cache_key
    }
}

pub struct RasterizedGlyph {
    data: Vec<u8>,
    width: u32,
    height: u32,
    left: i32,
    top: i32,
    is_color: bool,
}

impl RasterizedGlyph {
    pub(crate) fn new(
        data: Vec<u8>,
        width: u32,
        height: u32,
        left: i32,
        top: i32,
        is_color: bool,
    ) -> Self {
        Self {
            data,
            width,
            height,
            left,
            top,
            is_color,
        }
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

    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub fn byte_len(&self) -> usize {
        self.data.len()
    }

    pub fn left(&self) -> i32 {
        self.left
    }

    pub fn top(&self) -> i32 {
        self.top
    }

    pub fn is_color(&self) -> bool {
        self.is_color
    }
}
