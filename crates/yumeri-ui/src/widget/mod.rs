macro_rules! layout_widget_methods {
    () => {
        pub fn child(mut self, child: impl Into<crate::element::Element>) -> Self {
            self.children.push(child.into());
            self
        }

        pub fn children(
            mut self,
            children: impl IntoIterator<Item = impl Into<crate::element::Element>>,
        ) -> Self {
            self.children.extend(children.into_iter().map(Into::into));
            self
        }

        pub fn width(mut self, w: impl Into<crate::style::Dimension>) -> Self {
            self.style.width = w.into();
            self
        }

        pub fn height(mut self, h: impl Into<crate::style::Dimension>) -> Self {
            self.style.height = h.into();
            self
        }

        pub fn padding(mut self, p: f32) -> Self {
            self.style.padding = crate::style::Edges::all(p);
            self
        }

        pub fn padding_symmetric(mut self, h: f32, v: f32) -> Self {
            self.style.padding = crate::style::Edges::symmetric(h, v);
            self
        }

        pub fn margin(mut self, m: f32) -> Self {
            self.style.margin = crate::style::Edges::all(m);
            self
        }

        pub fn gap(mut self, g: f32) -> Self {
            self.style.gap = g;
            self
        }

        pub fn align_items(mut self, a: crate::style::Align) -> Self {
            self.style.align_items = Some(a);
            self
        }

        pub fn justify_content(mut self, j: crate::style::Justify) -> Self {
            self.style.justify_content = Some(j);
            self
        }

        pub fn background(mut self, color: yumeri_types::Color) -> Self {
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

        pub fn on_click(mut self, callback: crate::callback::AnyCallback) -> Self {
            self.event_handlers
                .push((crate::event::EventKind::Click, callback));
            self
        }

        pub fn focusable(mut self, f: bool) -> Self {
            self.focusable = f;
            self
        }
    };
}

macro_rules! impl_into_element {
    ($widget:ident, $variant:ident) => {
        impl From<$widget> for crate::element::Element {
            fn from(w: $widget) -> Self {
                crate::element::Element {
                    key: None,
                    kind: crate::element::ElementKind::Widget(
                        Box::new(crate::element::WidgetElement {
                            widget_type: crate::element::WidgetType::$variant,
                            style: w.style,
                            props: crate::element::WidgetProps::default(),
                            children: w.children,
                            event_handlers: w.event_handlers,
                            focusable: w.focusable,
                        }),
                    ),
                }
            }
        }
    };
}

mod column;
mod container;
mod image;
mod row;
mod shape;
mod stack;
mod text;

pub use column::Column;
pub use container::Container;
pub use image::Image;
pub use row::Row;
pub use shape::{CircleWidget, EllipseWidget, RectWidget, RoundedRectWidget};
pub use stack::Stack;
pub use text::Text;
