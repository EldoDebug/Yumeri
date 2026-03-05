use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::window::WindowId;

use crate::application::AppRequest;
use crate::delegate::WindowDelegate;

/// A wrapper around [`winit::window::Window`].
pub struct Window {
    inner: winit::window::Window,
}

impl Window {
    pub(crate) fn new(inner: winit::window::Window) -> Self {
        Self { inner }
    }

    pub fn id(&self) -> WindowId {
        self.inner.id()
    }

    pub fn title(&self) -> String {
        self.inner.title()
    }

    pub fn set_title(&self, title: &str) {
        self.inner.set_title(title);
    }

    pub fn surface_size(&self) -> PhysicalSize<u32> {
        self.inner.inner_size()
    }

    pub fn request_redraw(&self) {
        self.inner.request_redraw();
    }

    pub fn winit_window(&self) -> &winit::window::Window {
        &self.inner
    }
}

/// Internal entry stored per window in the [`Runner`](crate::application::Runner).
pub(crate) struct WindowEntry {
    pub(crate) window: Window,
    pub(crate) delegate: Option<Box<dyn WindowDelegate>>,
    pub(crate) render_state: Option<yumeri_renderer::WindowRenderState>,
}

/// Builder for creating windows with a fluent API.
pub struct WindowBuilder {
    pub(crate) attrs: winit::window::WindowAttributes,
    pub(crate) delegate: Option<Box<dyn WindowDelegate>>,
    pub(crate) enable_renderer_2d: bool,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            attrs: winit::window::Window::default_attributes(),
            delegate: None,
            enable_renderer_2d: false,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.attrs = self.attrs.with_title(title);
        self
    }

    pub fn with_surface_size(mut self, width: u32, height: u32) -> Self {
        self.attrs = self
            .attrs
            .with_inner_size(LogicalSize::new(width, height));
        self
    }

    pub fn with_min_surface_size(mut self, width: u32, height: u32) -> Self {
        self.attrs = self
            .attrs
            .with_min_inner_size(LogicalSize::new(width, height));
        self
    }

    pub fn with_max_surface_size(mut self, width: u32, height: u32) -> Self {
        self.attrs = self
            .attrs
            .with_max_inner_size(LogicalSize::new(width, height));
        self
    }

    pub fn with_position(mut self, x: i32, y: i32) -> Self {
        self.attrs = self.attrs.with_position(PhysicalPosition::new(x, y));
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.attrs = self.attrs.with_resizable(resizable);
        self
    }

    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.attrs = self.attrs.with_decorations(decorations);
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.attrs = self.attrs.with_transparent(transparent);
        self
    }

    pub fn with_visible(mut self, visible: bool) -> Self {
        self.attrs = self.attrs.with_visible(visible);
        self
    }

    pub fn with_delegate(mut self, delegate: impl WindowDelegate + 'static) -> Self {
        self.delegate = Some(Box::new(delegate));
        self
    }

    pub fn with_renderer_2d(mut self) -> Self {
        self.enable_renderer_2d = true;
        self
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Context passed to [`WindowDelegate`] callbacks.
///
/// Provides read-only access to the window and the ability to request
/// window creation, closure, or application exit via a deferred request queue.
pub struct WindowContext<'a> {
    window: &'a Window,
    requests: &'a mut Vec<AppRequest>,
}

impl<'a> WindowContext<'a> {
    pub(crate) fn new(window: &'a Window, requests: &'a mut Vec<AppRequest>) -> Self {
        Self { window, requests }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn create_window(&mut self, builder: WindowBuilder) {
        self.requests
            .push(AppRequest::CreateWindow(Box::new(builder)));
    }

    pub fn close_window(&mut self, id: WindowId) {
        self.requests.push(AppRequest::CloseWindow(id));
    }

    pub fn exit(&mut self) {
        self.requests.push(AppRequest::Exit);
    }
}
