use std::collections::HashMap;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

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
}

impl Runner {
    fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        builder: WindowBuilder,
    ) -> Result<WindowId, AppError> {
        let winit_window = event_loop.create_window(builder.attrs)?;
        let id = winit_window.id();
        let entry = WindowEntry {
            window: Window::new(winit_window),
            delegate: builder.delegate,
        };
        self.windows.insert(id, entry);
        Ok(id)
    }

    fn process_requests(&mut self, event_loop: &ActiveEventLoop) {
        while !self.requests.is_empty() {
            let requests: Vec<AppRequest> = self.requests.drain(..).collect();
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
                        if self.windows.remove(&id).is_some()
                            && let Some(delegate) = &mut self.delegate
                        {
                            let mut ctx = AppContext {
                                windows: &self.windows,
                                requests: &mut self.requests,
                            };
                            delegate.on_window_destroyed(&mut ctx, id);
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
        // Take the delegate out to avoid borrow conflicts
        let Some(entry) = self.windows.get_mut(&window_id) else {
            return;
        };
        let mut delegate = entry.delegate.take();

        if let Some(d) = &mut delegate {
            // Handle CloseRequested separately to avoid borrow issues on window removal
            if matches!(event, WindowEvent::CloseRequested) {
                let response = {
                    let entry = self.windows.get(&window_id).unwrap();
                    let mut ctx = WindowContext::new(&entry.window, &mut self.requests);
                    d.on_close_requested(&mut ctx)
                };
                if response == CloseResponse::Close {
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
                let entry = self.windows.get(&window_id).unwrap();
                let mut ctx = WindowContext::new(&entry.window, &mut self.requests);

                match event {
                    WindowEvent::Resized(size) => {
                        d.on_resized(&mut ctx, size);
                    }
                    WindowEvent::RedrawRequested => {
                        d.on_redraw_requested(&mut ctx);
                    }
                    WindowEvent::KeyboardInput {
                        event: ref key_event,
                        ..
                    } => {
                        let is_pressed = key_event.state.is_pressed();
                        d.on_key_input(&mut ctx, key_event, is_pressed);
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

        // Put the delegate back if the window still exists
        if let Some(entry) = self.windows.get_mut(&window_id) {
            entry.delegate = delegate;
        }

        self.process_requests(event_loop);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.process_requests(event_loop);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(delegate) = &mut self.delegate {
            let mut ctx = AppContext {
                windows: &self.windows,
                requests: &mut self.requests,
            };
            delegate.on_stop(&mut ctx);
        }
    }
}
