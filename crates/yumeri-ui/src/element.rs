use std::any::TypeId;

use yumeri_renderer::{Color, TextureId};

use crate::callback::AnyCallback;
use crate::event::EventKind;
use crate::style::Style;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ElementKey {
    Index(usize),
    Named(String),
}

pub struct Element {
    pub key: Option<ElementKey>,
    pub kind: ElementKind,
}

pub enum ElementKind {
    Widget(WidgetElement),
    Component(ComponentElement),
}

pub struct WidgetElement {
    pub widget_type: WidgetType,
    pub style: Style,
    pub props: WidgetProps,
    pub children: Vec<Element>,
    pub event_handlers: Vec<(EventKind, AnyCallback)>,
    pub focusable: bool,
}

pub struct ComponentElement {
    pub type_id: TypeId,
    pub create: Box<dyn FnOnce() -> Box<dyn std::any::Any>>,
    pub key: Option<ElementKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetType {
    Container,
    Column,
    Row,
    Stack,
    Text,
    Button,
    Image,
    TextInput,
    Checkbox,
    ScrollView,
}

#[derive(Clone, Debug, Default)]
pub struct WidgetProps {
    pub text: Option<String>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub text_color: Option<Color>,
    pub texture_id: Option<TextureId>,
    pub placeholder: Option<String>,
    pub checked: Option<bool>,
    pub scroll_offset: Option<[f32; 2]>,
    pub max_width: Option<f32>,
}
