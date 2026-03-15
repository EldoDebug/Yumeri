use yumeri_types::Color;

use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::style::{Dimension, Edges, Style};

pub struct Text {
    props: WidgetProps,
    style: Style,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        let content = text.into();
        Self {
            props: WidgetProps {
                text: Some(content),
                font_size: Some(16.0),
                line_height: Some(20.0),
                text_color: Some(Color::WHITE),
                ..Default::default()
            },
            style: Style::default(),
        }
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.props.font_size = Some(size);
        self.props.line_height = Some(size * 1.25);
        self
    }

    pub fn line_height(mut self, lh: f32) -> Self {
        self.props.line_height = Some(lh);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.props.text_color = Some(color);
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

    pub fn padding(mut self, p: f32) -> Self {
        self.style.padding = Edges::all(p);
        self
    }

    pub fn margin(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }

    pub fn opacity(mut self, o: f32) -> Self {
        self.style.opacity = o;
        self
    }

    pub fn flex_grow(mut self, g: f32) -> Self {
        self.style.flex_grow = g;
        self
    }

    pub fn flex_shrink(mut self, s: f32) -> Self {
        self.style.flex_shrink = s;
        self
    }
}

impl From<Text> for Element {
    fn from(t: Text) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(Box::new(WidgetElement {
                widget_type: WidgetType::Text,
                style: t.style,
                props: t.props,
                children: Vec::new(),
                event_handlers: Vec::new(),
                focusable: false,
            })),
        }
    }
}
