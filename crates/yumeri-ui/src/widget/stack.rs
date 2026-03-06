use yumeri_renderer::Color;

use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::style::{Dimension, Edges, Position, Style};

pub struct Stack {
    style: Style,
    children: Vec<Element>,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            children: Vec::new(),
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
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Stack> for Element {
    fn from(s: Stack) -> Self {
        // Stack children should all be position: absolute so they overlap
        let children: Vec<Element> = s.children;

        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::Stack,
                style: s.style,
                props: WidgetProps::default(),
                children,
                event_handlers: Vec::new(),
                focusable: false,
            }),
        }
    }
}

// Helper to wrap a child as absolute positioned
#[allow(dead_code)]
pub fn absolute(child: impl Into<Element>) -> Element {
    let mut elem = child.into();
    if let ElementKind::Widget(ref mut w) = elem.kind {
        w.style.position = Position::Absolute;
        w.style.inset = Edges::ZERO;
    }
    elem
}
