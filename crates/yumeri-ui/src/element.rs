use std::any::TypeId;

use yumeri_renderer::TextureId;
use yumeri_types::Color;

use crate::callback::AnyCallback;
use crate::component::{Component, ComponentBox};
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
    pub(crate) create: Box<dyn FnOnce() -> ComponentBox>,
}

impl Element {
    pub fn component<C: Component>(create: impl FnOnce() -> C + 'static) -> Self {
        Self {
            key: None,
            kind: ElementKind::Component(ComponentElement {
                type_id: TypeId::of::<C>(),
                create: Box::new(move || ComponentBox::new(create())),
            }),
        }
    }

    pub fn component_with_key<C: Component>(
        key: impl Into<String>,
        create: impl FnOnce() -> C + 'static,
    ) -> Self {
        Self {
            key: Some(ElementKey::Named(key.into())),
            kind: ElementKind::Component(ComponentElement {
                type_id: TypeId::of::<C>(),
                create: Box::new(move || ComponentBox::new(create())),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetType {
    Container,
    Column,
    Row,
    Stack,
    Text,
    Image,
}

impl WidgetType {
    pub fn is_text_bearing(self) -> bool {
        matches!(self, Self::Text)
    }
}

#[derive(Clone, Debug, Default)]
pub struct WidgetProps {
    pub text: Option<String>,
    pub font_size: Option<f32>,
    pub line_height: Option<f32>,
    pub text_color: Option<Color>,
    pub texture_id: Option<TextureId>,
    pub scroll_offset: Option<[f32; 2]>,
    pub max_width: Option<f32>,
}
