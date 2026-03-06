pub mod wayland;

use raw_window_handle::{RawDisplayHandle, RawWindowHandle};

use crate::error::Result;

#[derive(Debug, Clone)]
pub enum BackendEvent {
    FrameRequest,
    KeyInput {
        keycode: u32,
        pressed: bool,
        time: u32,
    },
    PointerMotion {
        x: f64,
        y: f64,
        time: u32,
    },
    PointerButton {
        button: u32,
        pressed: bool,
        time: u32,
    },
    Resize {
        width: u32,
        height: u32,
    },
    Shutdown,
}

pub trait Backend {
    fn dispatch(&mut self) -> Result<()>;
    fn next_event(&mut self) -> Option<BackendEvent>;
    fn raw_display_handle(&self) -> RawDisplayHandle;
    fn raw_window_handle(&self) -> RawWindowHandle;
    fn output_size(&self) -> (u32, u32);
    fn present(&mut self);
}
