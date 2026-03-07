use crate::callback::AnyCallback;
use crate::element::Element;
use crate::event::EventKind;
use crate::style::{Direction, Style};

pub struct Row {
    style: Style,
    children: Vec<Element>,
    event_handlers: Vec<(EventKind, AnyCallback)>,
    focusable: bool,
}

impl Row {
    pub fn new() -> Self {
        Self {
            style: Style {
                direction: Direction::Row,
                ..Default::default()
            },
            children: Vec::new(),
            event_handlers: Vec::new(),
            focusable: false,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.style.direction = Direction::Row;
        self
    }

    layout_widget_methods!();
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl_into_element!(Row, Row);
