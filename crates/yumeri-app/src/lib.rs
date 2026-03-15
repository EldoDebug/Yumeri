mod application;
mod delegate;
mod error;
mod input_conv;
mod window;

pub use application::{Application, ApplicationBuilder};
pub use delegate::{AppContext, AppDelegate, CloseResponse, WindowDelegate};
pub use error::AppError;
pub use window::{FullscreenMode, PresentMode, Window, WindowBuilder, WindowContext};

pub use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
pub use winit::window::WindowId;

pub use yumeri_input::{
    self, ButtonState, InputEvent, InputMap, InputState, InputTrigger, Key, KeyCode,
    KeyboardEvent, Modifiers, MouseButton, NamedKey, PointerEvent, PointerEventKind,
};

pub use yumeri_renderer::{self, Circle, Grayscale, PostEffect, PostEffectChain, Rect, RenderContext2D, RoundedRect};
pub use yumeri_threading::{self, ThreadPool, Task, TaskStatus, TaskError};
pub use yumeri_types::{Color, ShapeType};
pub use yumeri_renderer::{Texture, TextureId, UvRect};
pub use yumeri_renderer::{Alignment, Font, FontAttrs, FontFamily, FontStyle, FontWeight, TextMetrics, TextStyle, WrapMode};
pub use yumeri_renderer::ui::{NodeId, Scene as UiScene, UiContext};

#[cfg(feature = "ui")]
pub use yumeri_ui;
