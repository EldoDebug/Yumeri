use yumeri_renderer::Color;

use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::style::{Dimension, Edges, Style};

pub struct ScrollView {
    style: Style,
    children: Vec<Element>,
    scroll_offset: [f32; 2],
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            style: Style {
                ..Default::default()
            },
            children: Vec::new(),
            scroll_offset: [0.0, 0.0],
        }
    }

    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Element>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    pub fn scroll_offset(mut self, x: f32, y: f32) -> Self {
        self.scroll_offset = [x, y];
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

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    pub fn flex_grow(mut self, g: f32) -> Self {
        self.style.flex_grow = g;
        self
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ScrollView> for Element {
    fn from(sv: ScrollView) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::ScrollView,
                style: sv.style,
                props: WidgetProps {
                    scroll_offset: Some(sv.scroll_offset),
                    ..Default::default()
                },
                children: sv.children,
                event_handlers: Vec::new(),
                focusable: false,
            }),
        }
    }
}
