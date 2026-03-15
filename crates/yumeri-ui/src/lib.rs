pub mod app;
pub mod callback;
pub mod component;
pub mod element;
pub mod event;
pub mod event_ctx;
pub mod layout;
pub mod reconciler;
pub mod renderer_bridge;
pub mod style;
pub mod template_provider;
pub mod template_view_builder;
pub mod transition;
pub mod tree;
pub mod view_ctx;
pub mod widget;

pub mod prelude {
    pub use crate::app::UiApp;
    pub use crate::callback::AnyCallback;
    pub use crate::component::Component;
    pub use crate::element::Element;
    pub use crate::event::{EventKind, EventPayload};
    pub use crate::event_ctx::EventCtx;
    pub use crate::style::{Align, Dimension, Direction, Edges, Justify, Position, Style, Theme};
    pub use crate::template_provider::TemplateProvider;
    pub use crate::tree::{UiNodeId, UiTree};
    pub use crate::view_ctx::ViewCtx;
    pub use crate::widget::*;

    pub use yumeri_types::Color;
}

// Convenience re-exports at crate root
pub use app::UiApp;
pub use component::Component;
pub use element::Element;
pub use style::Style;
pub use template_provider::TemplateProvider;
pub use tree::UiTree;
pub use view_ctx::ViewCtx;
