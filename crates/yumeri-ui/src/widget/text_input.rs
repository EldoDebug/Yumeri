use yumeri_types::Color;

use crate::callback::AnyCallback;
use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::event::EventKind;
use crate::style::{Dimension, Edges, Style};

pub struct TextInput {
    props: WidgetProps,
    style: Style,
    event_handlers: Vec<(EventKind, AnyCallback)>,
}

impl TextInput {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        Self {
            props: WidgetProps {
                text: Some(value),
                font_size: Some(16.0),
                line_height: Some(20.0),
                text_color: Some(Color::rgb(0.93, 0.93, 0.93)),
                placeholder: None,
                ..Default::default()
            },
            style: Style {
                padding: Edges::symmetric(12.0, 8.0),
                corner_radius: 4.0,
                background: Some(Color::rgb(0.15, 0.15, 0.18)),
                border_color: Some(Color::rgb(0.3, 0.3, 0.35)),
                border_width: 1.0,
                height: Dimension::Px(36.0),
                ..Default::default()
            },
            event_handlers: Vec::new(),
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.props.placeholder = Some(text.into());
        self
    }

    pub fn on_change(mut self, callback: AnyCallback) -> Self {
        self.event_handlers.push((EventKind::TextInput, callback));
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.props.font_size = Some(size);
        self.props.line_height = Some(size * 1.25);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.props.text_color = Some(color);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.style.border_color = Some(color);
        self
    }

    pub fn width(mut self, w: impl Into<Dimension>) -> Self {
        self.style.width = w.into();
        self
    }

    pub fn height(mut self, h: impl Into<Dimension>) -> Self {
        self.style.height = h.into();
        self
    }

    pub fn corner_radius(mut self, r: f32) -> Self {
        self.style.corner_radius = r;
        self
    }

    pub fn margin(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }
}

impl From<TextInput> for Element {
    fn from(ti: TextInput) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::TextInput,
                style: ti.style,
                props: ti.props,
                children: Vec::new(),
                event_handlers: ti.event_handlers,
                focusable: true,
            }),
        }
    }
}
