use std::collections::HashMap;

use yumeri_font::{GlyphCacheKey, RasterizedGlyph};

use super::store::TextureStore;
use super::{TextureId, UvRect};
use crate::error::Result;
use crate::gpu::GpuContext;
use crate::text::LayoutCache;

const ATLAS_SIZE: u32 = 1024;
const ATLAS_BYTES: usize = (ATLAS_SIZE * ATLAS_SIZE * 4) as usize;
const PADDING: u32 = 1;

#[derive(Clone, Copy, Debug)]
pub struct CachedGlyph {
    pub uv: UvRect,
    pub size: [f32; 2],
    pub offset: [f32; 2],
    pub is_color: bool,
}

pub struct GlyphCache {
    pub(crate) layout_cache: LayoutCache,
    atlas_data: Vec<u8>,
    cache: HashMap<GlyphCacheKey, CachedGlyph>,
    atlas_texture_id: Option<TextureId>,
    cursor_x: u32,
    cursor_y: u32,
    row_height: u32,
    dirty: bool,
    atlas_generation: u64,
}

impl GlyphCache {
    pub fn new() -> Self {
        Self {
            layout_cache: LayoutCache::new(),
            atlas_data: vec![0u8; ATLAS_BYTES],
            cache: HashMap::new(),
            atlas_texture_id: None,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            dirty: false,
            atlas_generation: 0,
        }
    }

    pub fn get(&self, key: GlyphCacheKey) -> Option<&CachedGlyph> {
        self.cache.get(&key)
    }

    pub fn get_or_insert(
        &mut self,
        key: GlyphCacheKey,
        rasterized: &RasterizedGlyph,
    ) -> &CachedGlyph {
        if self.cache.contains_key(&key) {
            return &self.cache[&key];
        }

        let glyph_w = rasterized.width();
        let glyph_h = rasterized.height();

        if glyph_w == 0 || glyph_h == 0 {
            self.cache.insert(
                key,
                CachedGlyph {
                    uv: UvRect {
                        u_min: 0.0,
                        v_min: 0.0,
                        u_max: 0.0,
                        v_max: 0.0,
                    },
                    size: [0.0, 0.0],
                    offset: [rasterized.left() as f32, rasterized.top() as f32],
                    is_color: rasterized.is_color(),
                },
            );
            return &self.cache[&key];
        }

        let padded_w = glyph_w + PADDING;
        let padded_h = glyph_h + PADDING;

        // Check if glyph fits in current row
        if self.cursor_x + padded_w > ATLAS_SIZE {
            // Move to next row
            self.cursor_y += self.row_height + PADDING;
            self.cursor_x = 0;
            self.row_height = 0;
        }

        // Check if atlas is full
        if self.cursor_y + padded_h > ATLAS_SIZE {
            log::warn!("Glyph atlas full ({ATLAS_SIZE}x{ATLAS_SIZE}), clearing and re-rasterizing");
            self.clear();
        }

        let x = self.cursor_x;
        let y = self.cursor_y;

        // Blit glyph data into atlas
        let data = rasterized.data();
        let is_color = rasterized.is_color();

        for row in 0..glyph_h {
            let dst_offset = ((y + row) * ATLAS_SIZE + x) as usize * 4;
            if is_color {
                // RGBA data - copy directly
                let src_offset = (row * glyph_w) as usize * 4;
                let src_end = src_offset + glyph_w as usize * 4;
                let dst_end = dst_offset + glyph_w as usize * 4;
                self.atlas_data[dst_offset..dst_end]
                    .copy_from_slice(&data[src_offset..src_end]);
            } else {
                // Grayscale alpha - convert to (255, 255, 255, alpha)
                for col in 0..glyph_w {
                    let src_idx = (row * glyph_w + col) as usize;
                    let dst_idx = dst_offset + col as usize * 4;
                    let alpha = data[src_idx];
                    self.atlas_data[dst_idx] = 255;
                    self.atlas_data[dst_idx + 1] = 255;
                    self.atlas_data[dst_idx + 2] = 255;
                    self.atlas_data[dst_idx + 3] = alpha;
                }
            }
        }

        self.cursor_x = x + padded_w;
        if padded_h > self.row_height {
            self.row_height = padded_h;
        }
        self.dirty = true;

        let atlas_f = ATLAS_SIZE as f32;
        let cached = CachedGlyph {
            uv: UvRect {
                u_min: x as f32 / atlas_f,
                v_min: y as f32 / atlas_f,
                u_max: (x + glyph_w) as f32 / atlas_f,
                v_max: (y + glyph_h) as f32 / atlas_f,
            },
            size: [glyph_w as f32, glyph_h as f32],
            offset: [rasterized.left() as f32, rasterized.top() as f32],
            is_color: rasterized.is_color(),
        };

        self.cache.insert(key, cached);
        &self.cache[&key]
    }

    pub fn flush(
        &mut self,
        gpu: &GpuContext,
        texture_store: &mut TextureStore,
    ) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        match self.atlas_texture_id {
            Some(id) => {
                texture_store.update_raw_rgba(gpu, id, ATLAS_SIZE, ATLAS_SIZE, &self.atlas_data)?;
            }
            None => {
                let id = texture_store.create_from_raw_rgba(
                    gpu,
                    ATLAS_SIZE,
                    ATLAS_SIZE,
                    &self.atlas_data,
                )?;
                self.atlas_texture_id = Some(id);
            }
        }

        self.dirty = false;
        Ok(())
    }

    pub fn clear(&mut self) {
        self.atlas_data.fill(0);
        self.cache.clear();
        self.layout_cache = LayoutCache::new();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.row_height = 0;
        self.dirty = true;
        self.atlas_generation += 1;
    }

    pub fn atlas_texture_id(&self) -> Option<TextureId> {
        self.atlas_texture_id
    }

    pub fn atlas_generation(&self) -> u64 {
        self.atlas_generation
    }

    /// Shape text and return its measured dimensions `(width, height)`.
    /// The shaping result is cached so that subsequent rendering of the
    /// same text+style avoids redundant work.
    pub fn measure_text(
        &mut self,
        font: &mut yumeri_font::Font,
        text: &str,
        style: &crate::text::TextStyle,
    ) -> (f32, f32) {
        // shape_and_cache_glyphs populates the layout cache with pre-computed
        // width and height, so we just read them back.
        crate::text::shape_and_cache_glyphs(font, text, style, self);
        let key = crate::text::compute_layout_key(font, text, style);
        self.layout_cache
            .get(&key)
            .map(|(_, w, h)| (w, h))
            .unwrap_or((0.0, 0.0))
    }
}
