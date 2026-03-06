pub(crate) mod error;
pub(crate) mod frame;
pub(crate) mod gpu;
pub(crate) mod graph;
pub(crate) mod renderer;
pub(crate) mod resource;
pub mod texture;
pub mod text;
pub mod ui;

mod context;
mod render_state;

pub use context::RenderContext2D;
pub use error::RendererError;
pub use gpu::GpuContext;
pub use render_state::WindowRenderState;
pub use renderer::renderer2d::{Circle, Color, Rect, RoundedRect};
pub use renderer::renderer2d::shapes::ShapeType;
pub use text::TextStyle;
pub use texture::{Texture, TextureId, UvRect};

pub use yumeri_font::{
    Alignment, Font, FontAttrs, FontFamily, FontStyle, FontWeight, TextBuffer, TextMetrics,
    WrapMode,
};
