use std::collections::HashMap;

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, KeyEvent, MouseButton};
use winit::window::WindowId;
use yumeri_renderer::RenderContext2D;
use yumeri_renderer::ui::UiContext;

use crate::application::AppRequest;
use crate::window::{Window, WindowBuilder, WindowContext, WindowEntry};

/// Application lifecycle delegate.
///
/// Handles app-level events such as startup, shutdown, and window management notifications.
pub trait AppDelegate {
    fn on_start(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    fn on_stop(&mut self, ctx: &mut AppContext) {
        let _ = ctx;
    }

    fn on_window_created(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let _ = (ctx, window_id);
    }

    fn on_window_destroyed(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        let _ = (ctx, window_id);
    }
}

/// Per-window event delegate.
///
/// Each window can have its own delegate to handle input, rendering, and lifecycle events.
pub trait WindowDelegate {
    fn on_close_requested(&mut self, ctx: &mut WindowContext) -> CloseResponse {
        let _ = ctx;
        CloseResponse::Close
    }

    fn on_resized(&mut self, ctx: &mut WindowContext, size: PhysicalSize<u32>) {
        let _ = (ctx, size);
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        let _ = ctx;
    }

    fn on_key_input(&mut self, ctx: &mut WindowContext, event: &KeyEvent, is_pressed: bool) {
        let _ = (ctx, event, is_pressed);
    }

    fn on_mouse_input(&mut self, ctx: &mut WindowContext, state: ElementState, button: MouseButton) {
        let _ = (ctx, state, button);
    }

    fn on_cursor_moved(&mut self, ctx: &mut WindowContext, position: PhysicalPosition<f64>) {
        let _ = (ctx, position);
    }

    fn on_focused(&mut self, ctx: &mut WindowContext, focused: bool) {
        let _ = (ctx, focused);
    }

    fn on_scale_factor_changed(&mut self, ctx: &mut WindowContext, scale_factor: f64) {
        let _ = (ctx, scale_factor);
    }

    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        let _ = ctx;
    }

    fn on_ui_setup(&mut self, ctx: &mut UiContext) {
        let _ = ctx;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseResponse {
    Close,
    Prevent,
}

/// Context passed to [`AppDelegate`] callbacks.
///
/// Provides access to all windows and the ability to create/close windows or exit the app.
pub struct AppContext<'a> {
    pub(crate) windows: &'a HashMap<WindowId, WindowEntry>,
    pub(crate) requests: &'a mut Vec<AppRequest>,
}

impl AppContext<'_> {
    pub fn create_window(&mut self, builder: WindowBuilder) {
        self.requests
            .push(AppRequest::CreateWindow(Box::new(builder)));
    }

    pub fn close_window(&mut self, id: WindowId) {
        self.requests.push(AppRequest::CloseWindow(id));
    }

    pub fn window(&self, id: WindowId) -> Option<&Window> {
        self.windows.get(&id).map(|entry| &entry.window)
    }

    pub fn windows(&self) -> impl Iterator<Item = (WindowId, &Window)> {
        self.windows
            .iter()
            .map(|(&id, entry)| (id, &entry.window))
    }

    pub fn exit(&mut self) {
        self.requests.push(AppRequest::Exit);
    }
}
