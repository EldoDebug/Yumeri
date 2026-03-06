use winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize};
use winit::window::{WindowId, WindowLevel};
use yumeri_renderer::texture::glyph_cache::GlyphCache;

use crate::application::AppRequest;
use crate::delegate::WindowDelegate;

/// VSync / presentation mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentMode {
    /// VSync ON (FIFO) – no tearing, higher input latency.
    VSync,
    /// Triple buffering (MAILBOX) – no tearing, low latency.
    #[default]
    Mailbox,
    /// VSync OFF (IMMEDIATE) – lowest latency, may tear.
    Immediate,
}

/// Fullscreen mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FullscreenMode {
    /// Borderless fullscreen on the current monitor.
    Borderless,
    /// Exclusive fullscreen – the monitor is driven at a dedicated video mode.
    Exclusive,
}

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

    pub fn set_fullscreen(&self, mode: Option<FullscreenMode>) {
        use winit::window::Fullscreen;
        let fullscreen = mode.and_then(|m| match m {
            FullscreenMode::Borderless => Some(Fullscreen::Borderless(None)),
            FullscreenMode::Exclusive => {
                let video_mode = self
                    .inner
                    .current_monitor()
                    .and_then(|m| m.video_modes().next());
                if video_mode.is_none() {
                    log::warn!("No video mode available for exclusive fullscreen; ignoring");
                }
                video_mode.map(Fullscreen::Exclusive)
            }
        });
        self.inner.set_fullscreen(fullscreen);
    }

    pub fn set_always_on_top(&self, on_top: bool) {
        self.inner.set_window_level(window_level(on_top));
    }

    pub fn set_maximized(&self, maximized: bool) {
        self.inner.set_maximized(maximized);
    }

    pub fn set_cursor_visible(&self, visible: bool) {
        self.inner.set_cursor_visible(visible);
    }

    pub fn set_window_icon(&self, rgba: Vec<u8>, width: u32, height: u32) {
        let icon = winit::window::Icon::from_rgba(rgba, width, height).ok();
        self.inner.set_window_icon(icon);
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
    pub(crate) ui_scene: Option<yumeri_renderer::ui::Scene>,
}

/// Builder for creating windows with a fluent API.
pub struct WindowBuilder {
    pub(crate) attrs: winit::window::WindowAttributes,
    pub(crate) delegate: Option<Box<dyn WindowDelegate>>,
    pub(crate) enable_renderer_2d: bool,
    pub(crate) enable_ui_renderer: bool,
    pub(crate) present_mode: PresentMode,
    pub(crate) transparent: bool,
    pub(crate) fullscreen: Option<FullscreenMode>,
    pub(crate) cursor_visible: Option<bool>,
    pub(crate) window_icon: Option<(Vec<u8>, u32, u32)>,
}

impl WindowBuilder {
    pub fn new() -> Self {
        Self {
            attrs: winit::window::Window::default_attributes(),
            delegate: None,
            enable_renderer_2d: false,
            enable_ui_renderer: false,
            present_mode: PresentMode::default(),
            transparent: false,
            fullscreen: None,
            cursor_visible: None,
            window_icon: None,
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
        self.transparent = transparent;
        self
    }

    pub fn with_fullscreen(mut self, mode: FullscreenMode) -> Self {
        self.fullscreen = Some(mode);
        self
    }

    pub fn with_present_mode(mut self, mode: PresentMode) -> Self {
        self.present_mode = mode;
        self
    }

    pub fn with_always_on_top(mut self, on_top: bool) -> Self {
        self.attrs = self.attrs.with_window_level(window_level(on_top));
        self
    }

    pub fn with_maximized(mut self, maximized: bool) -> Self {
        self.attrs = self.attrs.with_maximized(maximized);
        self
    }

    pub fn with_cursor_visible(mut self, visible: bool) -> Self {
        self.cursor_visible = Some(visible);
        self
    }

    pub fn with_window_icon(mut self, rgba: Vec<u8>, width: u32, height: u32) -> Self {
        self.window_icon = Some((rgba, width, height));
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

    pub fn with_ui_renderer(mut self) -> Self {
        self.enable_ui_renderer = true;
        self
    }
}

impl Default for WindowBuilder {
    fn default() -> Self {
        Self::new()
    }
}

fn window_level(on_top: bool) -> WindowLevel {
    if on_top {
        WindowLevel::AlwaysOnTop
    } else {
        WindowLevel::Normal
    }
}

/// Context passed to [`WindowDelegate`] callbacks.
///
/// Provides read-only access to the window and the ability to request
/// window creation, closure, or application exit via a deferred request queue.
pub struct WindowContext<'a> {
    window: &'a Window,
    requests: &'a mut Vec<AppRequest>,
    ui_scene: Option<&'a mut yumeri_renderer::ui::Scene>,
    glyph_cache: Option<&'a mut GlyphCache>,
}

impl<'a> WindowContext<'a> {
    pub(crate) fn new(
        window: &'a Window,
        requests: &'a mut Vec<AppRequest>,
        ui_scene: Option<&'a mut yumeri_renderer::ui::Scene>,
        glyph_cache: Option<&'a mut GlyphCache>,
    ) -> Self {
        Self {
            window,
            requests,
            ui_scene,
            glyph_cache,
        }
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

    pub fn ui_scene(&mut self) -> Option<&mut yumeri_renderer::ui::Scene> {
        self.ui_scene.as_deref_mut()
    }

    pub fn glyph_cache(&mut self) -> Option<&mut GlyphCache> {
        self.glyph_cache.as_deref_mut()
    }

    pub fn ui_scene_and_glyph_cache(
        &mut self,
    ) -> (Option<&mut yumeri_renderer::ui::Scene>, Option<&mut GlyphCache>) {
        (self.ui_scene.as_deref_mut(), self.glyph_cache.as_deref_mut())
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
