use yumeri_renderer::Color;

use crate::callback::AnyCallback;
use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::event::EventKind;
use crate::style::{Dimension, Edges, Style};

pub struct Checkbox {
    label: Option<String>,
    props: WidgetProps,
    style: Style,
    event_handlers: Vec<(EventKind, AnyCallback)>,
}

impl Checkbox {
    pub fn new(checked: bool) -> Self {
        Self {
            label: None,
            props: WidgetProps {
                checked: Some(checked),
                font_size: Some(16.0),
                text_color: Some(Color::WHITE),
                ..Default::default()
            },
            style: Style {
                width: Dimension::Px(20.0),
                height: Dimension::Px(20.0),
                corner_radius: 4.0,
                background: Some(if checked {
                    Color::rgb(0.25, 0.46, 0.85)
                } else {
                    Color::rgb(0.2, 0.2, 0.24)
                }),
                border_color: Some(Color::rgb(0.3, 0.3, 0.35)),
                border_width: 1.0,
                ..Default::default()
            },
            event_handlers: Vec::new(),
        }
    }

    pub fn label(mut self, text: impl Into<String>) -> Self {
        let label = text.into();
        self.props.text = Some(label.clone());
        self.label = Some(label);
        self
    }

    pub fn on_toggle(mut self, callback: AnyCallback) -> Self {
        self.event_handlers.push((EventKind::Click, callback));
        self
    }

    pub fn margin(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }
}

impl From<Checkbox> for Element {
    fn from(cb: Checkbox) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::Checkbox,
                style: cb.style,
                props: cb.props,
                children: Vec::new(),
                event_handlers: cb.event_handlers,
                focusable: true,
            }),
        }
    }
}
