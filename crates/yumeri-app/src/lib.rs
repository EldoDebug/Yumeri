mod application;
mod delegate;
mod error;
mod window;

pub use application::{Application, ApplicationBuilder};
pub use delegate::{AppContext, AppDelegate, CloseResponse, WindowDelegate};
pub use error::AppError;
pub use window::{Window, WindowBuilder, WindowContext};

pub use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
pub use winit::event::{ElementState, KeyEvent, MouseButton};
pub use winit::keyboard::{Key, KeyCode, NamedKey, PhysicalKey};
pub use winit::window::WindowId;

pub use yumeri_renderer::{self, Circle, Rect, RenderContext2D, RoundedRect};
pub use yumeri_types::{Color, ShapeType};
pub use yumeri_renderer::{Texture, TextureId, UvRect};
pub use yumeri_renderer::{Alignment, Font, FontAttrs, FontFamily, FontStyle, FontWeight, TextMetrics, TextStyle, WrapMode};
pub use yumeri_renderer::ui::{NodeId, Scene as UiScene, UiContext};

#[cfg(feature = "ui")]
pub use yumeri_ui;
