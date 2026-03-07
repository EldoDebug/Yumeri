use crate::callback::AnyCallback;
use crate::element::Element;
use crate::event::EventKind;
use crate::style::Style;

pub struct Container {
    style: Style,
    children: Vec<Element>,
    event_handlers: Vec<(EventKind, AnyCallback)>,
    focusable: bool,
}

impl Container {
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

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

impl_into_element!(Container, Container);
