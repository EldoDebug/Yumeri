use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use yumeri_font::{Font, TextBuffer, TextMetrics};

use crate::renderer::renderer2d::shapes::Rect;
use crate::renderer::renderer2d::Color;
use crate::texture::glyph_cache::{CachedGlyph, GlyphCache};
use crate::texture::Texture;

const MAX_LAYOUT_CACHE_ENTRIES: usize = 256;

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub color: Color,
    pub font_attrs: yumeri_font::FontAttrs,
    pub max_width: Option<f32>,
    pub wrap: yumeri_font::WrapMode,
    pub alignment: yumeri_font::Alignment,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            line_height: 20.0,
            color: Color::WHITE,
            font_attrs: yumeri_font::FontAttrs::new(),
            max_width: None,
            wrap: yumeri_font::WrapMode::Word,
            alignment: yumeri_font::Alignment::Left,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct LayoutGlyph {
    pub x: f32,
    pub y: f32,
    pub cached: CachedGlyph,
    pub is_color: bool,
}

impl LayoutGlyph {
    pub fn to_rect(&self, origin: [f32; 2], style_color: Color, texture: Option<Texture>) -> Rect {
        let half_w = self.cached.size[0] / 2.0;
        let half_h = self.cached.size[1] / 2.0;
        let color = if self.is_color { Color::WHITE } else { style_color };
        Rect {
            position: [origin[0] + self.x + half_w, origin[1] + self.y + half_h],
            size: [half_w, half_h],
            color,
            texture,
        }
    }
}

// Hash-based key to avoid per-frame String allocation
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct LayoutCacheKey(u64);

fn compute_layout_key(font: &Font, text: &str, style: &TextStyle) -> LayoutCacheKey {
    let mut hasher = std::hash::DefaultHasher::new();
    font.id().hash(&mut hasher);
    text.hash(&mut hasher);
    style.font_size.to_bits().hash(&mut hasher);
    style.line_height.to_bits().hash(&mut hasher);
    style.max_width.map(|w| w.to_bits()).hash(&mut hasher);
    style.wrap.hash(&mut hasher);
    style.alignment.hash(&mut hasher);
    style.font_attrs.hash(&mut hasher);
    LayoutCacheKey(hasher.finish())
}

pub(crate) struct LayoutCache {
    entries: HashMap<LayoutCacheKey, Vec<LayoutGlyph>>,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

use crate::texture::TextureId;

pub(crate) fn shape_and_cache_glyphs<'a>(
    font: &mut Font,
    text: &str,
    style: &TextStyle,
    glyph_cache: &'a mut GlyphCache,
) -> (&'a [LayoutGlyph], Option<TextureId>) {
    let key = compute_layout_key(font, text, style);

    if glyph_cache.layout_cache.entries.contains_key(&key) {
        let atlas_id = glyph_cache.atlas_texture_id();
        return (&glyph_cache.layout_cache.entries[&key], atlas_id);
    }

    // Slow path: shape, rasterize, and cache
    let metrics = TextMetrics::new(style.font_size, style.line_height);
    let mut buffer = TextBuffer::new(font, metrics);

    if let Some(max_w) = style.max_width {
        buffer.set_size(font, Some(max_w), None);
    }
    buffer.set_wrap(font, style.wrap);
    buffer.set_alignment(style.alignment);
    buffer.set_text(font, text, &style.font_attrs);

    let glyphs = buffer.shape_and_layout(font);
    let mut result = Vec::with_capacity(glyphs.len());

    for glyph in &glyphs {
        let glyph_key = glyph.cache_key();

        let cached = if let Some(&cached) = glyph_cache.get(glyph_key) {
            cached
        } else {
            let Some(rasterized) = buffer.rasterize(font, glyph) else {
                continue;
            };
            if rasterized.width() == 0 || rasterized.height() == 0 {
                continue;
            }
            *glyph_cache.get_or_insert(glyph_key, &rasterized)
        };

        if cached.size[0] == 0.0 || cached.size[1] == 0.0 {
            continue;
        }

        result.push(LayoutGlyph {
            x: glyph.x + cached.offset[0],
            y: glyph.y - cached.offset[1],
            cached,
            is_color: cached.is_color,
        });
    }

    // Evict oldest entries if cache is full
    if glyph_cache.layout_cache.entries.len() >= MAX_LAYOUT_CACHE_ENTRIES {
        glyph_cache.layout_cache.entries.clear();
    }

    glyph_cache.layout_cache.entries.insert(key, result);
    let atlas_id = glyph_cache.atlas_texture_id();
    (&glyph_cache.layout_cache.entries[&key], atlas_id)
}
