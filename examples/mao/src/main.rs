use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::{Window, WindowAttributes, WindowId};
use yumeri_live2d::{Live2DModel, MotionPriority, StdFsLoader};
use yumeri_renderer::live2d::{coords, Live2DRenderer};
use yumeri_renderer::{GpuContext, SwapchainConfig, WindowRenderState};

struct App {
    window: Option<&'static Window>,
    state: Option<State>,
    last_frame: Instant,
    cursor_pos: Option<(f64, f64)>,
    captured: bool,
}

struct State {
    gpu: GpuContext,
    render_state: WindowRenderState,
    model: Live2DModel,
    renderer: Option<Live2DRenderer>,
}

impl State {
    fn destroy(&mut self) {
        self.render_state.destroy(&self.gpu);
        if let Some(renderer) = self.renderer.take() {
            renderer.destroy(&self.gpu);
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            state: None,
            last_frame: Instant::now(),
            cursor_pos: None,
            captured: false,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);

        let window = event_loop
            .create_window(WindowAttributes::default().with_title("Yumeri - Mao (Live2D)"))
            .expect("create_window");
        let window: &'static Window = Box::leak(Box::new(window));
        let size = window.inner_size();

        let display_handle = window.display_handle().unwrap().as_raw();
        let window_handle = window.window_handle().unwrap().as_raw();

        let gpu = GpuContext::new(display_handle, window_handle).expect("GpuContext");

        let render_state = WindowRenderState::new(
            &gpu,
            display_handle,
            window_handle,
            size.width.max(1),
            size.height.max(1),
            false,
            false,
            SwapchainConfig::default(),
        )
        .expect("WindowRenderState");

        let model3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/Mao/Mao.model3.json");
        let mut model =
            Live2DModel::load(&StdFsLoader, &model3_path).expect("load Live2D model");

        let swapchain_format = render_state.swapchain_format();
        let renderer =
            Live2DRenderer::from_model(&gpu, swapchain_format, &mut model).expect("Live2DRenderer");

        window.request_redraw();

        self.window = Some(window);
        self.state = Some(State {
            gpu,
            render_state,
            model,
            renderer: Some(renderer),
        });
        self.last_frame = Instant::now();
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => {
                state.destroy();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Err(e) = state.render_state.on_resize(&state.gpu, size.width, size.height) {
                    eprintln!("Resize error: {e}");
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = Some((position.x, position.y));
                if self.captured {
                    let (w, h) = state.render_state.swapchain_extent();
                    let v = coords::pixel_to_view(position.x, position.y, w, h);
                    state.model.set_dragging(v.x, v.y);
                }
            }
            WindowEvent::MouseInput {
                state: s, button, ..
            } => {
                if button == MouseButton::Left {
                    match s {
                        ElementState::Pressed => {
                            self.captured = true;
                            if let Some((x, y)) = self.cursor_pos {
                                let (w, h) = state.render_state.swapchain_extent();
                                let v = coords::pixel_to_view(x, y, w, h);
                                state.model.set_dragging(v.x, v.y);
                                on_click(state, x, y);
                            }
                        }
                        ElementState::Released => {
                            self.captured = false;
                            state.model.set_dragging(0.0, 0.0);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = state.renderer.as_mut() else {
                    return;
                };
                let now = Instant::now();
                let dt = (now - self.last_frame).as_secs_f32().min(1.0 / 10.0);
                self.last_frame = now;

                let _ = state.model.update(dt);

                let (w, h) = state.render_state.swapchain_extent();
                let projection = coords::compute_projection_fit(w, h);
                let mvp = projection * state.model.model_matrix().mat4();

                let gpu = &state.gpu;
                let model = &mut state.model;

                let result = state.render_state.render_frame(
                    gpu,
                    |_ctx| {},
                    Some(&mut |builder, backbuffer| {
                        if let Err(e) =
                            renderer.register_pass(gpu, builder, backbuffer, model, mvp)
                        {
                            eprintln!("Live2D render error: {e}");
                        }
                    }),
                    None,
                );

                if let Err(e) = result {
                    eprintln!("Render error: {e}");
                }
            }
            _ => {}
        }
    }
}

fn on_click(state: &mut State, x: f64, y: f64) {
    let (w, h) = state.render_state.swapchain_extent();
    let v = coords::pixel_to_view(x, y, w, h);

    if state.model.hit_test_view_space("Head", v.x, v.y) {
        let _ = state.model.set_random_expression();
    } else if state.model.hit_test_view_space("Body", v.x, v.y) {
        let _ = state
            .model
            .start_random_motion("TapBody", MotionPriority::Normal);
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    yumeri_live2d::core::install_tracing_logger();

    let event_loop = EventLoop::new().expect("event_loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("run");
}
