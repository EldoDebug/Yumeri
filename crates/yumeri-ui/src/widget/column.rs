use crate::callback::AnyCallback;
use crate::element::Element;
use crate::event::EventKind;
use crate::style::{Direction, Style};

pub struct Column {
    style: Style,
    children: Vec<Element>,
    event_handlers: Vec<(EventKind, AnyCallback)>,
    focusable: bool,
}

impl Column {
    pub fn new() -> Self {
        Self {
            style: Style {
                direction: Direction::Column,
                ..Default::default()
            },
            children: Vec::new(),
            event_handlers: Vec::new(),
            focusable: false,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.style.direction = Direction::Column;
        self
    }

    layout_widget_methods!();
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl_into_element!(Column, Column);
