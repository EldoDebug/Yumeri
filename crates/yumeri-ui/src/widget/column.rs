use yumeri_types::Color;

use crate::element::{Element, ElementKind, WidgetElement, WidgetProps, WidgetType};
use crate::style::{Align, Dimension, Direction, Edges, Justify, Style};

pub struct Column {
    style: Style,
    children: Vec<Element>,
}

impl Column {
    pub fn new() -> Self {
        Self {
            style: Style {
                direction: Direction::Column,
                ..Default::default()
            },
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

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self.style.direction = Direction::Column;
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

    pub fn gap(mut self, g: f32) -> Self {
        self.style.gap = g;
        self
    }

    pub fn align_items(mut self, a: Align) -> Self {
        self.style.align_items = Some(a);
        self
    }

    pub fn justify_content(mut self, j: Justify) -> Self {
        self.style.justify_content = Some(j);
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.style.background = Some(color);
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

    pub fn flex_grow(mut self, g: f32) -> Self {
        self.style.flex_grow = g;
        self
    }

    pub fn flex_shrink(mut self, s: f32) -> Self {
        self.style.flex_shrink = s;
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Column> for Element {
    fn from(c: Column) -> Self {
        Element {
            key: None,
            kind: ElementKind::Widget(WidgetElement {
                widget_type: WidgetType::Column,
                style: c.style,
                props: WidgetProps::default(),
                children: c.children,
                event_handlers: Vec::new(),
                focusable: false,
            }),
        }
    }
}
