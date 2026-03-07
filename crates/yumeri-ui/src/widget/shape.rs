macro_rules! define_shape_widget {
    ($name:ident, $variant:ident) => {
        pub struct $name {
            pub style: crate::style::Style,
            children: Vec<crate::element::Element>,
            event_handlers: Vec<(crate::event::EventKind, crate::callback::AnyCallback)>,
            focusable: bool,
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    style: crate::style::Style::default(),
                    children: Vec::new(),
                    event_handlers: Vec::new(),
                    focusable: false,
                }
            }

            layout_widget_methods!();
        }

        impl_into_element!($name, $variant);
    };
}

define_shape_widget!(RectWidget, Rect);
define_shape_widget!(RoundedRectWidget, RoundedRect);
define_shape_widget!(CircleWidget, Circle);
define_shape_widget!(EllipseWidget, Ellipse);
