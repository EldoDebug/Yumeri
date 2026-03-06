use yumeri_renderer::TextureId;

use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::style::{Dimension, Edges, Style};

pub struct Image {
    texture_id: TextureId,
    style: Style,
}

impl Image {
    pub fn new(texture_id: TextureId) -> Self {
        Self {
            texture_id,
            style: Style::default(),
        }
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

    pub fn opacity(mut self, o: f32) -> Self {
        self.style.opacity = o;
        self
    }

    pub fn margin(mut self, m: f32) -> Self {
        self.style.margin = Edges::all(m);
        self
    }
}

impl From<Image> for Element {
    fn from(img: Image) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::Image,
                style: img.style,
                props: WidgetProps {
                    texture_id: Some(img.texture_id),
                    ..Default::default()
                },
                children: Vec::new(),
                event_handlers: Vec::new(),
                focusable: false,
            }),
        }
    }
}
