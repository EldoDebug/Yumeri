use std::fmt;

use crate::attrs::{
    Alignment, FontAttrs, FontFamily, FontStretch, FontStyle, FontWeight, TextMetrics, WrapMode,
};
use crate::font::Font;
use crate::shaped_glyph::{GlyphCacheKey, RasterizedGlyph, ShapedGlyph};

pub struct TextBuffer {
    buffer: cosmic_text::Buffer,
    swash_cache: cosmic_text::SwashCache,
    alignment: Option<cosmic_text::Align>,
    cached_layout_height: f32,
}

impl TextBuffer {
    pub fn new(font: &mut Font, metrics: TextMetrics) -> Self {
        let cosmic_metrics = cosmic_text::Metrics::new(metrics.font_size, metrics.line_height);
        Self {
            buffer: cosmic_text::Buffer::new(&mut font.inner, cosmic_metrics),
            swash_cache: cosmic_text::SwashCache::new(),
            alignment: None,
            cached_layout_height: 0.0,
        }
    }

    pub fn set_size(&mut self, font: &mut Font, width: Option<f32>, height: Option<f32>) {
        self.buffer.set_size(&mut font.inner, width, height);
    }

    pub fn set_text(&mut self, font: &mut Font, text: &str, attrs: &FontAttrs) {
        let cosmic_attrs = to_cosmic_attrs(attrs);
        self.buffer.set_text(
            &mut font.inner,
            text,
            &cosmic_attrs,
            cosmic_text::Shaping::Advanced,
            self.alignment,
        );
    }

    pub fn set_rich_text<'a>(
        &mut self,
        font: &mut Font,
        spans: impl IntoIterator<Item = (&'a str, FontAttrs)>,
        default_attrs: &FontAttrs,
    ) {
        let default_cosmic = to_cosmic_attrs(default_attrs);
        let spans: Vec<(&'a str, FontAttrs)> = spans.into_iter().collect();

        self.buffer.set_rich_text(
            &mut font.inner,
            spans.iter().map(|(text, attrs)| (*text, to_cosmic_attrs(attrs))),
            &default_cosmic,
            cosmic_text::Shaping::Advanced,
            self.alignment,
        );
    }

    pub fn set_wrap(&mut self, font: &mut Font, wrap: WrapMode) {
        self.buffer.set_wrap(&mut font.inner, to_cosmic_wrap(wrap));
    }

    pub fn set_alignment(&mut self, alignment: Alignment) {
        self.alignment = Some(to_cosmic_align(alignment));
    }

    pub fn set_metrics(&mut self, font: &mut Font, metrics: TextMetrics) {
        let cosmic_metrics = cosmic_text::Metrics::new(metrics.font_size, metrics.line_height);
        self.buffer.set_metrics(&mut font.inner, cosmic_metrics);
    }

    pub fn shape_and_layout(&mut self, font: &mut Font) -> Vec<ShapedGlyph> {
        self.buffer.shape_until_scroll(&mut font.inner, true);

        let mut glyphs = Vec::new();
        let mut height = 0.0f32;
        for run in self.buffer.layout_runs() {
            height = height.max(run.line_y + run.line_height);
            for glyph in run.glyphs.iter() {
                let physical = glyph.physical((0., 0.), 1.0);
                glyphs.push(ShapedGlyph {
                    x: glyph.x,
                    y: run.line_y,
                    width: glyph.w,
                    font_size: glyph.font_size,
                    line_index: run.line_i,
                    color: glyph.color_opt.map(from_cosmic_color),
                    cache_key: GlyphCacheKey(physical.cache_key),
                });
            }
        }
        self.cached_layout_height = height;
        glyphs
    }

    pub fn rasterize(
        &mut self,
        font: &mut Font,
        glyph: &ShapedGlyph,
    ) -> Option<RasterizedGlyph> {
        let image = self
            .swash_cache
            .get_image_uncached(&mut font.inner, glyph.cache_key.0)?;

        let pixel_count = (image.placement.width * image.placement.height) as usize;
        let is_color = image.data.len() == pixel_count * 4;

        Some(RasterizedGlyph::new(
            image.data,
            image.placement.width,
            image.placement.height,
            image.placement.left,
            image.placement.top,
            is_color,
        ))
    }

    pub fn layout_height(&self) -> f32 {
        self.cached_layout_height
    }
}

fn to_cosmic_family(family: &FontFamily) -> cosmic_text::Family<'_> {
    match family {
        FontFamily::Name(name) => cosmic_text::Family::Name(name),
        FontFamily::Serif => cosmic_text::Family::Serif,
        FontFamily::SansSerif => cosmic_text::Family::SansSerif,
        FontFamily::Monospace => cosmic_text::Family::Monospace,
        FontFamily::Cursive => cosmic_text::Family::Cursive,
        FontFamily::Fantasy => cosmic_text::Family::Fantasy,
    }
}

fn to_cosmic_weight(weight: FontWeight) -> cosmic_text::Weight {
    cosmic_text::Weight(weight.0)
}

fn to_cosmic_style(style: FontStyle) -> cosmic_text::Style {
    match style {
        FontStyle::Normal => cosmic_text::Style::Normal,
        FontStyle::Italic => cosmic_text::Style::Italic,
        FontStyle::Oblique => cosmic_text::Style::Oblique,
    }
}

fn to_cosmic_stretch(stretch: FontStretch) -> cosmic_text::Stretch {
    match stretch {
        FontStretch::UltraCondensed => cosmic_text::Stretch::UltraCondensed,
        FontStretch::ExtraCondensed => cosmic_text::Stretch::ExtraCondensed,
        FontStretch::Condensed => cosmic_text::Stretch::Condensed,
        FontStretch::SemiCondensed => cosmic_text::Stretch::SemiCondensed,
        FontStretch::Normal => cosmic_text::Stretch::Normal,
        FontStretch::SemiExpanded => cosmic_text::Stretch::SemiExpanded,
        FontStretch::Expanded => cosmic_text::Stretch::Expanded,
        FontStretch::ExtraExpanded => cosmic_text::Stretch::ExtraExpanded,
        FontStretch::UltraExpanded => cosmic_text::Stretch::UltraExpanded,
    }
}

fn to_cosmic_attrs(attrs: &FontAttrs) -> cosmic_text::Attrs<'_> {
    cosmic_text::Attrs::new()
        .family(to_cosmic_family(&attrs.family))
        .weight(to_cosmic_weight(attrs.weight))
        .style(to_cosmic_style(attrs.style))
        .stretch(to_cosmic_stretch(attrs.stretch))
}

fn to_cosmic_wrap(wrap: WrapMode) -> cosmic_text::Wrap {
    match wrap {
        WrapMode::Word => cosmic_text::Wrap::Word,
        WrapMode::Glyph => cosmic_text::Wrap::Glyph,
        WrapMode::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
        WrapMode::None => cosmic_text::Wrap::None,
    }
}

fn to_cosmic_align(alignment: Alignment) -> cosmic_text::Align {
    match alignment {
        Alignment::Left => cosmic_text::Align::Left,
        Alignment::Center => cosmic_text::Align::Center,
        Alignment::Right => cosmic_text::Align::Right,
        Alignment::Justified => cosmic_text::Align::Justified,
        Alignment::End => cosmic_text::Align::End,
    }
}

fn from_cosmic_color(color: cosmic_text::Color) -> [u8; 4] {
    [color.r(), color.g(), color.b(), color.a()]
}

impl fmt::Debug for TextBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextBuffer").finish_non_exhaustive()
    }
}
