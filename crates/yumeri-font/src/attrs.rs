#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontFamily {
    Name(String),
    Serif,
    SansSerif,
    Monospace,
    Cursive,
    Fantasy,
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::SansSerif
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const NORMAL: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMI_BOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
    Oblique,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontStretch {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    #[default]
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

#[derive(Clone, Debug, Default)]
pub struct FontAttrs {
    pub(crate) family: FontFamily,
    pub(crate) weight: FontWeight,
    pub(crate) style: FontStyle,
    pub(crate) stretch: FontStretch,
}

impl FontAttrs {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn family(mut self, family: FontFamily) -> Self {
        self.family = family;
        self
    }

    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    pub fn get_family(&self) -> &FontFamily {
        &self.family
    }

    pub fn get_weight(&self) -> FontWeight {
        self.weight
    }

    pub fn get_style(&self) -> FontStyle {
        self.style
    }

    pub fn get_stretch(&self) -> FontStretch {
        self.stretch
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TextMetrics {
    pub font_size: f32,
    pub line_height: f32,
}

impl TextMetrics {
    pub fn new(font_size: f32, line_height: f32) -> Self {
        debug_assert!(font_size > 0.0, "font_size must be positive");
        debug_assert!(line_height > 0.0, "line_height must be positive");
        Self {
            font_size,
            line_height,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WrapMode {
    #[default]
    Word,
    Glyph,
    WordOrGlyph,
    None,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justified,
    End,
}
