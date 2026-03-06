use std::collections::HashMap;

use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;
use yumeri_renderer::GpuContext;

use crate::delegate::{AppContext, AppDelegate, CloseResponse};
use crate::error::AppError;
use crate::window::{Window, WindowBuilder, WindowContext, WindowEntry};

/// Top-level application entry point.
///
/// Use [`Application::builder`] to configure and run the application.
pub struct Application;

impl Application {
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder {
            delegate: None,
            initial_windows: Vec::new(),
        }
    }
}

/// Configures and launches the application with a fluent builder API.
pub struct ApplicationBuilder {
    delegate: Option<Box<dyn AppDelegate>>,
    initial_windows: Vec<WindowBuilder>,
}

impl ApplicationBuilder {
    pub fn with_delegate(mut self, delegate: impl AppDelegate + 'static) -> Self {
        self.delegate = Some(Box::new(delegate));
        self
    }

    pub fn with_window(mut self, builder: WindowBuilder) -> Self {
        self.initial_windows.push(builder);
        self
    }

    pub fn run(self) -> Result<(), AppError> {
        let event_loop = EventLoop::new()?;
        let mut runner = Runner {
            delegate: self.delegate,
            windows: HashMap::new(),
            pending_builders: self.initial_windows,
            requests: Vec::new(),
            gpu_context: None,
        };
        event_loop.run_app(&mut runner)?;
        Ok(())
    }
}

// -- Internal Runner --

pub(crate) enum AppRequest {
    CreateWindow(Box<WindowBuilder>),
    CloseWindow(WindowId),
    Exit,
}

struct Runner {
    delegate: Option<Box<dyn AppDelegate>>,
    windows: HashMap<WindowId, WindowEntry>,
    pending_builders: Vec<WindowBuilder>,
    requests: Vec<AppRequest>,
    gpu_context: Option<GpuContext>,
}

impl Runner {
    fn ensure_gpu_context(&mut self, winit_window: &winit::window::Window) {
        if self.gpu_context.is_some() {
            return;
        }
        let display_handle = winit_window.display_handle().unwrap().as_raw();
        let window_handle = winit_window.window_handle().unwrap().as_raw();
        match GpuContext::new(display_handle, window_handle) {
            Ok(gpu) => self.gpu_context = Some(gpu),
            Err(e) => eprintln!("Failed to create GPU context: {e}"),
        }
    }

    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        builder: WindowBuilder,
    ) -> Result<WindowId, AppError> {
        let enable_2d = builder.enable_renderer_2d;
        let enable_ui = builder.enable_ui_renderer;
        let needs_rendering = enable_2d || enable_ui;

        let winit_window = event_loop.create_window(builder.attrs)?;
        let id = winit_window.id();

        let render_state = if needs_rendering {
            self.ensure_gpu_context(&winit_window);
            if let Some(gpu) = &self.gpu_context {
                let display_handle = winit_window.display_handle().unwrap().as_raw();
                let window_handle = winit_window.window_handle().unwrap().as_raw();
                let size = winit_window.inner_size();
                match yumeri_renderer::WindowRenderState::new(
                    gpu,
                    display_handle,
                    window_handle,
                    size.width,
                    size.height,
                    enable_2d,
                    enable_ui,
                ) {
                    Ok(state) => Some(state),
                    Err(e) => {
                        eprintln!("Failed to create render state: {e}");
                        None
                    }
                }
            } else {
                None
            }
        } else {
            None
        };

        let ui_scene = if enable_ui {
            Some(yumeri_renderer::ui::Scene::new())
        } else {
            None
        };

        let mut entry = WindowEntry {
            window: Window::new(winit_window),
            delegate: builder.delegate,
            render_state,
            ui_scene,
        };

        // Call on_ui_setup for UI-enabled windows
        let surface_size = entry.window.surface_size();
        if let (Some(d), Some(scene)) = (&mut entry.delegate, &mut entry.ui_scene) {
            if let (Some(rs), Some(gpu)) = (&mut entry.render_state, &self.gpu_context) {
                let mut ui_ctx = rs.setup_ui_context(
                    scene,
                    gpu,
                    (surface_size.width, surface_size.height),
                );
                d.on_ui_setup(&mut ui_ctx);
            } else {
                let mut ui_ctx = yumeri_renderer::ui::UiContext::new(
                    scene,
                    (surface_size.width, surface_size.height),
                );
                d.on_ui_setup(&mut ui_ctx);
            }
        }

        if entry.ui_scene.as_ref().is_some_and(|s| s.is_dirty()) {
            entry.window.request_redraw();
        }

        self.windows.insert(id, entry);
        Ok(id)
    }

    fn process_requests(&mut self, event_loop: &ActiveEventLoop) {
        while !self.requests.is_empty() {
            let requests = std::mem::take(&mut self.requests);
            for request in requests {
                match request {
                    AppRequest::CreateWindow(builder) => {
                        match self.create_window(event_loop, *builder) {
                            Ok(id) => {
                                if let Some(delegate) = &mut self.delegate {
                                    let mut ctx = AppContext {
                                        windows: &self.windows,
                                        requests: &mut self.requests,
                                    };
                                    delegate.on_window_created(&mut ctx, id);
                                }
                            }
                            Err(e) => eprintln!("Failed to create window: {e}"),
                        }
                    }
                    AppRequest::CloseWindow(id) => {
                        if let Some(mut entry) = self.windows.remove(&id) {
                            if let (Some(gpu), Some(rs)) =
                                (&self.gpu_context, &mut entry.render_state)
                            {
                                rs.destroy(gpu);
                            }
                            if let Some(delegate) = &mut self.delegate {
                                let mut ctx = AppContext {
                                    windows: &self.windows,
                                    requests: &mut self.requests,
                                };
                                delegate.on_window_destroyed(&mut ctx, id);
                            }
                        }
                    }
                    AppRequest::Exit => {
                        event_loop.exit();
                    }
                }
            }
        }
    }
}

impl ApplicationHandler for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let builders: Vec<WindowBuilder> = self.pending_builders.drain(..).collect();
        let mut created_ids = Vec::new();

        for builder in builders {
            match self.create_window(event_loop, builder) {
                Ok(id) => created_ids.push(id),
                Err(e) => eprintln!("Failed to create window: {e}"),
            }
        }

        if let Some(delegate) = &mut self.delegate {
            let mut ctx = AppContext {
                windows: &self.windows,
                requests: &mut self.requests,
            };
            delegate.on_start(&mut ctx);

            for id in created_ids {
                let mut ctx = AppContext {
                    windows: &self.windows,
                    requests: &mut self.requests,
                };
                delegate.on_window_created(&mut ctx, id);
            }
        }

        self.process_requests(event_loop);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return;
        };
        let mut delegate = entry.delegate.take();
        let mut render_state = entry.render_state.take();
        let mut ui_scene = entry.ui_scene.take();

        if let Some(d) = &mut delegate {
            if matches!(event, WindowEvent::CloseRequested) {
                let response = {
                    let entry = self.windows.get(&window_id).unwrap();
                    let mut ctx =
                        WindowContext::new(&entry.window, &mut self.requests, ui_scene.as_mut());
                    d.on_close_requested(&mut ctx)
                };
                if response == CloseResponse::Close {
                    if let (Some(gpu), Some(rs)) = (&self.gpu_context, &mut render_state) {
                        rs.destroy(gpu);
                    }
                    self.windows.remove(&window_id);

                    if let Some(app_delegate) = &mut self.delegate {
                        let mut app_ctx = AppContext {
                            windows: &self.windows,
                            requests: &mut self.requests,
                        };
                        app_delegate.on_window_destroyed(&mut app_ctx, window_id);
                    }

                    self.process_requests(event_loop);
                    return;
                }
            } else {
                // GPU render operations for specific events
                match &event {
                    WindowEvent::RedrawRequested => {
                        if let (Some(gpu), Some(rs)) = (&self.gpu_context, &mut render_state) {
                            let result = rs.render_frame(
                                gpu,
                                |ctx| {
                                    d.on_render2d(ctx);
                                },
                                ui_scene.as_mut(),
                            );
                            if let Err(e) = result {
                                eprintln!("Render error: {e}");
                            }
                        }
                    }
                    WindowEvent::Resized(size) => {
                        if let (Some(gpu), Some(rs)) = (&self.gpu_context, &mut render_state) {
                            if let Err(e) = rs.on_resize(gpu, size.width, size.height) {
                                eprintln!("Resize error: {e}");
                            }
                        }
                    }
                    _ => {}
                }

                // Delegate dispatch with shared context
                let entry = self.windows.get(&window_id).unwrap();
                let mut ctx =
                    WindowContext::new(&entry.window, &mut self.requests, ui_scene.as_mut());
                match event {
                    WindowEvent::RedrawRequested => d.on_redraw_requested(&mut ctx),
                    WindowEvent::Resized(size) => d.on_resized(&mut ctx, size),
                    WindowEvent::KeyboardInput {
                        event: key_event, ..
                    } => {
                        let is_pressed = key_event.state.is_pressed();
                        d.on_key_input(&mut ctx, &key_event, is_pressed);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        d.on_mouse_input(&mut ctx, state, button);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        d.on_cursor_moved(&mut ctx, position);
                    }
                    WindowEvent::Focused(focused) => {
                        d.on_focused(&mut ctx, focused);
                    }
                    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        d.on_scale_factor_changed(&mut ctx, scale_factor);
                    }
                    _ => {}
                }
            }
        }

        // Auto request_redraw when UI scene is dirty
        if ui_scene.as_ref().is_some_and(|s| s.is_dirty())
            && let Some(entry) = self.windows.get(&window_id)
        {
            entry.window.request_redraw();
        }

        if let Some(entry) = self.windows.get_mut(&window_id) {
            entry.delegate = delegate;
            entry.render_state = render_state;
            entry.ui_scene = ui_scene;
        }

        self.process_requests(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_requests(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // Destroy render states before GPU context is dropped
        for (_, entry) in &mut self.windows {
            if let (Some(gpu), Some(rs)) = (&self.gpu_context, &mut entry.render_state) {
                rs.destroy(gpu);
            }
        }

        if let Some(delegate) = &mut self.delegate {
            let mut ctx = AppContext {
                windows: &self.windows,
                requests: &mut self.requests,
            };
            delegate.on_stop(&mut ctx);
        }
    }
}
