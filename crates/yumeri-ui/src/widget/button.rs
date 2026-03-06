use yumeri_types::Color;

use crate::callback::AnyCallback;
use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::event::EventKind;
use crate::style::{Align, Dimension, Edges, Justify, Style};
use crate::transition::TransitionDef;

pub struct Button {
    props: WidgetProps,
    style: Style,
    event_handlers: Vec<(EventKind, AnyCallback)>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        let label = label.into();
        Self {
            props: WidgetProps {
                text: Some(label),
                font_size: Some(16.0),
                line_height: Some(20.0),
                text_color: Some(Color::WHITE),
                ..Default::default()
            },
            style: Style {
                padding: Edges::symmetric(16.0, 8.0),
                corner_radius: 6.0,
                background: Some(Color::rgb(0.25, 0.46, 0.85)),
                align_items: Some(Align::Center),
                justify_content: Some(Justify::Center),
                ..Default::default()
            },
            event_handlers: Vec::new(),
        }
    }

    pub fn on_click(mut self, callback: AnyCallback) -> Self {
        self.event_handlers.push((EventKind::Click, callback));
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
        self
    }

    pub fn text_color(mut self, color: Color) -> Self {
        self.props.text_color = Some(color);
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.props.font_size = Some(size);
        self.props.line_height = Some(size * 1.25);
        self
    }

    pub fn corner_radius(mut self, r: f32) -> Self {
        self.style.corner_radius = r;
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

    pub fn padding_symmetric(mut self, h: f32, v: f32) -> Self {
        self.style.padding = Edges::symmetric(h, v);
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

    pub fn transition(mut self, t: TransitionDef) -> Self {
        self.style.transitions.push(t);
        self
    }

    pub fn flex_grow(mut self, g: f32) -> Self {
        self.style.flex_grow = g;
        self
    }
}

impl From<Button> for Element {
    fn from(b: Button) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::Button,
                style: b.style,
                props: b.props,
                children: Vec::new(),
                event_handlers: b.event_handlers,
                focusable: true,
            }),
        }
    }
}
