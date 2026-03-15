use yumeri_types::Color;

use crate::transition::TransitionDef;

#[derive(Clone, Debug)]
pub struct Style {
    // Layout -> taffy
    pub direction: Direction,
    pub width: Dimension,
    pub height: Dimension,
    pub min_width: Dimension,
    pub min_height: Dimension,
    pub max_width: Dimension,
    pub max_height: Dimension,
    pub padding: Edges,
    pub margin: Edges,
    pub gap: f32,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: Dimension,
    pub align_items: Option<Align>,
    pub align_self: Option<Align>,
    pub justify_content: Option<Justify>,
    pub position: Position,
    pub inset: Edges,

    // Visual -> Scene
    pub background: Option<Color>,
    /// Reserved — parsed from templates but not yet consumed by the renderer.
    pub border_color: Option<Color>,
    pub border_width: f32,
    pub corner_radius: f32,
    pub opacity: f32,
    pub visible: bool,

    // Transform
    pub translate: [f32; 2],
    pub scale: [f32; 2],
    pub rotation: f32,
    /// Reserved — the renderer always uses center origin [0.5, 0.5].
    pub transform_origin: [f32; 2],

    /// Reserved — parsed from templates but not yet wired to the animation runtime.
    pub transitions: Vec<TransitionDef>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            direction: Direction::Column,
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_width: Dimension::Auto,
            min_height: Dimension::Auto,
            max_width: Dimension::Auto,
            max_height: Dimension::Auto,
            padding: Edges::ZERO,
            margin: Edges::ZERO,
            gap: 0.0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: Dimension::Auto,
            align_items: None,
            align_self: None,
            justify_content: None,
            position: Position::Relative,
            inset: Edges::ZERO,

            background: None,
            border_color: None,
            border_width: 0.0,
            corner_radius: 0.0,
            opacity: 1.0,
            visible: true,

            translate: [0.0, 0.0],
            scale: [1.0, 1.0],
            rotation: 0.0,
            transform_origin: [0.5, 0.5],

            transitions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dimension {
    Auto,
    Px(f32),
    Percent(f32),
}

impl From<f32> for Dimension {
    fn from(v: f32) -> Self {
        Dimension::Px(v)
    }
}

impl From<i32> for Dimension {
    fn from(v: i32) -> Self {
        Dimension::Px(v as f32)
    }
}

impl From<u32> for Dimension {
    fn from(v: u32) -> Self {
        Dimension::Px(v as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn all(v: f32) -> Self {
        Self {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    Relative,
    Absolute,
}

#[derive(Clone, Debug)]
pub struct Theme {
    pub primary: Color,
    pub on_primary: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub text_secondary: Color,
    pub error: Color,
    pub font_size: f32,
    pub corner_radius: f32,
    pub spacing: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            primary: Color::rgb(0.25, 0.46, 0.85),
            on_primary: Color::WHITE,
            surface: Color::rgb(0.15, 0.15, 0.18),
            on_surface: Color::rgb(0.9, 0.9, 0.9),
            background: Color::rgb(0.1, 0.1, 0.12),
            border: Color::rgb(0.3, 0.3, 0.35),
            text: Color::rgb(0.93, 0.93, 0.93),
            text_secondary: Color::rgb(0.6, 0.6, 0.65),
            error: Color::rgb(0.85, 0.25, 0.25),
            font_size: 16.0,
            corner_radius: 6.0,
            spacing: 8.0,
        }
    }
}
