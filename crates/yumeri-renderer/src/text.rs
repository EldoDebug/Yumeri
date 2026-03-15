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
    /// Advance width from the original shaped glyph (used for layout measurement).
    pub advance_width: f32,
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
pub(crate) struct LayoutCacheKey(u64);

pub(crate) fn compute_layout_key(font: &Font, text: &str, style: &TextStyle) -> LayoutCacheKey {
    LayoutCacheKey(hash_text_style_core(font, text, style))
}

/// Hash all fields that affect text layout and shaping (excluding color).
fn hash_text_style_core(font: &Font, text: &str, style: &TextStyle) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    font.id().hash(&mut hasher);
    text.hash(&mut hasher);
    style.font_size.to_bits().hash(&mut hasher);
    style.line_height.to_bits().hash(&mut hasher);
    style.max_width.map(|w| w.to_bits()).hash(&mut hasher);
    style.wrap.hash(&mut hasher);
    style.alignment.hash(&mut hasher);
    style.font_attrs.hash(&mut hasher);
    hasher.finish()
}

/// Fingerprint covering layout + visual appearance (includes color).
pub(crate) fn compute_text_fingerprint(font: &Font, text: &str, style: &TextStyle) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    hash_text_style_core(font, text, style).hash(&mut hasher);
    style.color.r.to_bits().hash(&mut hasher);
    style.color.g.to_bits().hash(&mut hasher);
    style.color.b.to_bits().hash(&mut hasher);
    style.color.a.to_bits().hash(&mut hasher);
    hasher.finish()
}

struct CachedLayout {
    glyphs: Vec<LayoutGlyph>,
    layout_width: f32,
    layout_height: f32,
    last_used: u64,
}

pub(crate) struct LayoutCache {
    entries: HashMap<LayoutCacheKey, CachedLayout>,
    generation: u64,
}

impl LayoutCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            generation: 0,
        }
    }

    fn contains(&self, key: &LayoutCacheKey) -> bool {
        self.entries.contains_key(key)
    }

    pub(crate) fn get(&self, key: &LayoutCacheKey) -> Option<(&[LayoutGlyph], f32, f32)> {
        self.entries
            .get(key)
            .map(|e| (e.glyphs.as_slice(), e.layout_width, e.layout_height))
    }

    /// Mark a key as recently used (O(1) via generation counter).
    fn touch(&mut self, key: &LayoutCacheKey) {
        self.generation += 1;
        if let Some(entry) = self.entries.get_mut(key) {
            entry.last_used = self.generation;
        }
    }

    fn insert(
        &mut self,
        key: LayoutCacheKey,
        glyphs: Vec<LayoutGlyph>,
        layout_width: f32,
        layout_height: f32,
    ) {
        if self.entries.len() >= MAX_LAYOUT_CACHE_ENTRIES {
            // Evict the least-recently-used half (O(n) via partial sort)
            let mut by_age: Vec<(LayoutCacheKey, u64)> = self
                .entries
                .iter()
                .map(|(&k, v)| (k, v.last_used))
                .collect();
            let half = by_age.len() / 2;
            by_age.select_nth_unstable_by_key(half, |&(_, age)| age);
            for &(evicted, _) in &by_age[..half] {
                self.entries.remove(&evicted);
            }
        }
        self.generation += 1;
        self.entries.insert(
            key,
            CachedLayout {
                glyphs,
                layout_width,
                layout_height,
                last_used: self.generation,
            },
        );
    }
}

use crate::texture::TextureId;

/// Shape text, rasterize glyphs into the atlas, and cache the layout.
/// Returns (layout_glyphs, atlas_texture_id, layout_height).
pub(crate) fn shape_and_cache_glyphs<'a>(
    font: &mut Font,
    text: &str,
    style: &TextStyle,
    glyph_cache: &'a mut GlyphCache,
) -> (&'a [LayoutGlyph], Option<TextureId>, f32) {
    let key = compute_layout_key(font, text, style);

    if glyph_cache.layout_cache.contains(&key) {
        glyph_cache.layout_cache.touch(&key);
        let atlas_id = glyph_cache.atlas_texture_id();
        let (glyphs, _width, height) = glyph_cache.layout_cache.get(&key).unwrap();
        return (glyphs, atlas_id, height);
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
    let layout_height = buffer.layout_height();
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
            advance_width: glyph.width,
        });
    }

    let layout_width = result
        .iter()
        .map(|g| (g.x - g.cached.offset[0]) + g.advance_width)
        .fold(0.0f32, f32::max);
    glyph_cache.layout_cache.insert(key, result, layout_width, layout_height);
    let atlas_id = glyph_cache.atlas_texture_id();
    let (glyphs, _width, height) = glyph_cache.layout_cache.get(&key).unwrap();
    (glyphs, atlas_id, height)
}
