use crate::callback::AnyCallback;
use crate::element::{Element, ElementKind};
use crate::event::EventKind;
use crate::style::{Edges, Position, Style};

pub struct Stack {
    style: Style,
    children: Vec<Element>,
    event_handlers: Vec<(EventKind, AnyCallback)>,
    focusable: bool,
}

impl Stack {
    pub fn new() -> Self {
        Self {
            style: Style::default(),
            children: Vec::new(),
            event_handlers: Vec::new(),
            focusable: false,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    layout_widget_methods!();
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

impl_into_element!(Stack, Stack);

#[allow(dead_code)]
pub fn absolute(child: impl Into<Element>) -> Element {
    let mut elem = child.into();
    if let ElementKind::Widget(ref mut w) = elem.kind {
        w.style.position = Position::Absolute;
        w.style.inset = Edges::ZERO;
    }
    elem
}
