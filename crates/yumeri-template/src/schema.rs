use std::collections::HashMap;
use serde::Deserialize;

use crate::animation::{AnimationDef, EasingKind};
use crate::binding::ValueOrBinding;
use crate::token::{TokenValue, ValueOrToken};

#[derive(Clone, Debug, Deserialize)]
pub struct Template {
    pub name: String,
    #[serde(default)]
    pub tokens: HashMap<String, TokenValue>,
    pub root: TemplateNode,
    #[serde(default)]
    pub animations: HashMap<String, AnimationDef>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TemplateNode {
    #[serde(default)]
    pub id: Option<String>,
    pub widget: WidgetKind,
    #[serde(default)]
    pub style: PartialStyle,
    #[serde(default)]
    pub props: Option<NodeProps>,
    #[serde(default)]
    pub visible: Option<ValueOrBinding<bool>>,
    #[serde(default)]
    pub states: HashMap<String, StateOverride>,
    #[serde(default)]
    pub transitions: Vec<TransitionSpec>,
    #[serde(default)]
    pub children: Vec<TemplateNode>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum WidgetKind {
    Container,
    Column,
    Row,
    Stack,
    Text,
    Image,
    Rect,
    RoundedRect,
    Circle,
    Ellipse,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct PartialStyle {
    // Layout
    pub direction: Option<DirectionKind>,
    pub width: Option<DimensionValue>,
    pub height: Option<DimensionValue>,
    pub min_width: Option<DimensionValue>,
    pub min_height: Option<DimensionValue>,
    pub max_width: Option<DimensionValue>,
    pub max_height: Option<DimensionValue>,
    pub padding: Option<EdgesValue>,
    pub margin: Option<EdgesValue>,
    pub gap: Option<ValueOrToken<f32>>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<DimensionValue>,
    pub align_items: Option<AlignKind>,
    pub align_self: Option<AlignKind>,
    pub justify_content: Option<JustifyKind>,
    pub position: Option<PositionKind>,
    pub inset: Option<EdgesValue>,

    // Visual
    pub background: Option<ValueOrToken<(f32, f32, f32, f32)>>,
    pub border_color: Option<ValueOrToken<(f32, f32, f32, f32)>>,
    pub border_width: Option<f32>,
    pub corner_radius: Option<ValueOrToken<f32>>,
    pub opacity: Option<f32>,
    pub visible: Option<bool>,

    // Transform
    pub translate: Option<[f32; 2]>,
    pub scale: Option<[f32; 2]>,
    pub rotation: Option<f32>,
    pub transform_origin: Option<[f32; 2]>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct StateOverride {
    pub background: Option<ValueOrToken<(f32, f32, f32, f32)>>,
    pub border_color: Option<ValueOrToken<(f32, f32, f32, f32)>>,
    pub border_width: Option<f32>,
    pub corner_radius: Option<ValueOrToken<f32>>,
    pub opacity: Option<f32>,
    pub text_color: Option<ValueOrToken<(f32, f32, f32, f32)>>,
    pub translate: Option<[f32; 2]>,
    pub scale: Option<[f32; 2]>,
    pub rotation: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TransitionSpec {
    pub property: TransitionPropertyKind,
    pub duration_ms: u64,
    pub easing: EasingKind,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum TransitionPropertyKind {
    Opacity,
    BackgroundColor,
    Width,
    Height,
    CornerRadius,
    Translate,
    Scale,
    Rotation,
}

#[derive(Clone, Debug, Deserialize)]
pub struct NodeProps {
    pub text: Option<ValueOrBinding<String>>,
    pub font_size: Option<ValueOrToken<f32>>,
    pub text_color: Option<ValueOrToken<(f32, f32, f32, f32)>>,
}

#[derive(Clone, Debug, Deserialize)]
pub enum DimensionValue {
    Auto,
    Px(ValueOrToken<f32>),
    Percent(f32),
}

#[derive(Clone, Debug, Deserialize)]
pub enum EdgesValue {
    All(ValueOrToken<f32>),
    Symmetric(ValueOrToken<f32>, ValueOrToken<f32>),
    Each {
        top: ValueOrToken<f32>,
        right: ValueOrToken<f32>,
        bottom: ValueOrToken<f32>,
        left: ValueOrToken<f32>,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum DirectionKind {
    Row,
    Column,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum AlignKind {
    Start,
    End,
    Center,
    Stretch,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum JustifyKind {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
pub enum PositionKind {
    Relative,
    Absolute,
}
