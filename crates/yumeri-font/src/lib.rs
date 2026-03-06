pub(crate) mod attrs;
pub(crate) mod error;
pub(crate) mod font;
pub(crate) mod shaped_glyph;
pub(crate) mod text_buffer;

pub use attrs::{
    Alignment, FontAttrs, FontFamily, FontStretch, FontStyle, FontWeight, TextMetrics, WrapMode,
};
pub use error::{FontError, Result};
pub use font::Font;
pub use shaped_glyph::{GlyphCacheKey, RasterizedGlyph, ShapedGlyph};
pub use text_buffer::TextBuffer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_creation_succeeds() {
        let _font = Font::new();
    }

    #[test]
    fn ascii_text_shaping() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(200.0), Some(100.0));
        buffer.set_text(&mut font, "Hello, world!", &FontAttrs::new());
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "ASCII text should produce glyphs");

        for glyph in &glyphs {
            assert!(glyph.width > 0.0, "glyph width should be positive");
        }
    }

    #[test]
    fn japanese_text_shaping() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(400.0), Some(100.0));
        buffer.set_text(
            &mut font,
            "\u{3053}\u{3093}\u{306b}\u{3061}\u{306f}",
            &FontAttrs::new(),
        );
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "Japanese text should produce glyphs");
    }

    #[test]
    fn emoji_rasterize_is_color() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(32.0, 40.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(200.0), Some(100.0));
        buffer.set_text(&mut font, "\u{1F980}", &FontAttrs::new());
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "emoji should produce glyphs");

        let rasterized = buffer.rasterize(&mut font, &glyphs[0]);
        if let Some(glyph) = rasterized {
            assert!(glyph.is_color(), "emoji glyph should be color (RGBA)");
            assert!(glyph.byte_len() > 0, "rasterized glyph should have data");
        }
    }

    #[test]
    fn arabic_text_shaping() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(400.0), Some(100.0));
        buffer.set_text(
            &mut font,
            "\u{0645}\u{0631}\u{062d}\u{0628}\u{0627}",
            &FontAttrs::new(),
        );
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "Arabic text should produce glyphs");
    }

    #[test]
    fn glyph_rasterize_non_empty() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(32.0, 40.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(200.0), Some(100.0));
        buffer.set_text(&mut font, "A", &FontAttrs::new());
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty());

        let rasterized = buffer.rasterize(&mut font, &glyphs[0]);
        if let Some(glyph) = rasterized {
            assert!(glyph.byte_len() > 0, "rasterized glyph should have data");
            assert!(glyph.width() > 0, "rasterized glyph should have width");
            assert!(glyph.height() > 0, "rasterized glyph should have height");
            assert!(!glyph.is_color(), "regular text glyph should not be color");
        }
    }

    #[test]
    fn layout_height_positive() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(200.0), Some(100.0));
        buffer.set_text(&mut font, "Hello\nWorld", &FontAttrs::new());
        buffer.shape_and_layout(&mut font);
        assert!(
            buffer.layout_height() > 0.0,
            "layout height should be positive"
        );
    }

    #[test]
    fn font_attrs_builder() {
        let attrs = FontAttrs::new()
            .family(FontFamily::Monospace)
            .weight(FontWeight::BOLD)
            .style(FontStyle::Italic)
            .stretch(FontStretch::Expanded);

        assert_eq!(*attrs.get_family(), FontFamily::Monospace);
        assert_eq!(attrs.get_weight(), FontWeight::BOLD);
        assert_eq!(attrs.get_style(), FontStyle::Italic);
        assert_eq!(attrs.get_stretch(), FontStretch::Expanded);
    }

    #[test]
    fn rich_text_shaping() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(400.0), Some(100.0));
        buffer.set_rich_text(
            &mut font,
            [
                ("Hello, ", FontAttrs::new()),
                ("world!", FontAttrs::new().weight(FontWeight::BOLD)),
            ],
            &FontAttrs::new(),
        );
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "rich text should produce glyphs");
    }

    #[test]
    fn load_font_data_invalid_returns_error() {
        let mut font = Font::new();
        let result = font.load_font_data(vec![0, 1, 2, 3]);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FontError::InvalidFontData));
    }

    #[test]
    fn load_font_nonexistent_returns_io_error() {
        let mut font = Font::new();
        let result = font.load_font("nonexistent_font.ttf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FontError::Io { .. }));
    }

    #[test]
    fn set_wrap_and_alignment() {
        let mut font = Font::new();
        let metrics = TextMetrics::new(16.0, 20.0);
        let mut buffer = TextBuffer::new(&mut font, metrics);
        buffer.set_size(&mut font, Some(100.0), Some(100.0));
        buffer.set_wrap(&mut font, WrapMode::Word);
        buffer.set_alignment(Alignment::Center);
        buffer.set_text(
            &mut font,
            "The quick brown fox jumps over the lazy dog",
            &FontAttrs::new(),
        );
        let glyphs = buffer.shape_and_layout(&mut font);
        assert!(!glyphs.is_empty(), "wrapped text should produce glyphs");
    }
}
