use std::time::Instant;

use yumeri_app::*;

struct MyApp;

impl AppDelegate for MyApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

struct MyWindow {
    fullscreen: bool,
    always_on_top: bool,
    maximized: bool,
    cursor_visible: bool,
    frame_count: u32,
    last_fps_time: Instant,
    fps: f64,
}

impl MyWindow {
    fn new() -> Self {
        Self {
            fullscreen: false,
            always_on_top: false,
            maximized: false,
            cursor_visible: true,
            frame_count: 0,
            last_fps_time: Instant::now(),
            fps: 0.0,
        }
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.last_fps_time.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            self.fps = self.frame_count as f64 / elapsed;
            self.frame_count = 0;
            self.last_fps_time = Instant::now();
            println!("FPS: {:.1}", self.fps);
        }
    }
}

impl WindowDelegate for MyWindow {
    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        self.update_fps();

        let (w, h) = ctx.surface_size();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;

        // No full-screen background — the window is transparent, so
        // the desktop shows through wherever nothing is drawn.

        // Floating shapes on a see-through window
        ctx.draw_rounded_rect(RoundedRect {
            position: [cx - 150.0, cy - 60.0],
            size: [300.0, 120.0],
            corner_radius: 24.0,
            color: Color::rgba(0.15, 0.15, 0.25, 0.85),
            texture: None,
        });

        let indicator_color = if self.always_on_top {
            Color::rgba(0.2, 0.9, 0.4, 0.9)
        } else {
            Color::rgba(0.9, 0.3, 0.3, 0.9)
        };
        ctx.draw_rounded_rect(RoundedRect {
            position: [cx - 130.0, cy - 40.0],
            size: [260.0, 30.0],
            corner_radius: 8.0,
            color: indicator_color,
            texture: None,
        });

        ctx.draw_circle(Circle {
            position: [cx, cy + 120.0],
            radius: 40.0,
            color: Color::rgba(0.2, 0.6, 0.9, 0.7),
            texture: None,
        });

        // FPS indicator bar — length proportional to FPS (1px per FPS)
        let bar_width = (self.fps as f32).min(w as f32);
        ctx.draw_rect(Rect {
            position: [0.0, 0.0],
            size: [bar_width, 4.0],
            color: Color::rgb(0.0, 1.0, 0.4),
            texture: None,
        });
    }

    fn on_redraw_requested(&mut self, _ctx: &mut WindowContext) {
        // Redraws are driven by ApplicationBuilder::with_target_fps().
        // Do NOT call request_redraw() here — that would bypass the FPS limiter.
    }

    fn on_input(&mut self, ctx: &mut WindowContext, event: &InputEvent) {
        let InputEvent::Keyboard(kb) = event else { return };
        if !kb.state.is_pressed() {
            return;
        }

        let window = ctx.window();

        match &kb.key {
            // [F] Toggle fullscreen (borderless)
            Key::Character(c) if c == "f" => {
                self.fullscreen = !self.fullscreen;
                if self.fullscreen {
                    window.set_fullscreen(Some(FullscreenMode::Borderless));
                    println!("[F] Fullscreen: Borderless ON");
                } else {
                    window.set_fullscreen(None);
                    println!("[F] Fullscreen: OFF");
                }
            }

            // [G] Toggle fullscreen (exclusive)
            Key::Character(c) if c == "g" => {
                self.fullscreen = !self.fullscreen;
                if self.fullscreen {
                    window.set_fullscreen(Some(FullscreenMode::Exclusive));
                    println!("[G] Fullscreen: Exclusive ON");
                } else {
                    window.set_fullscreen(None);
                    println!("[G] Fullscreen: OFF");
                }
            }

            // [T] Toggle always-on-top
            Key::Character(c) if c == "t" => {
                self.always_on_top = !self.always_on_top;
                window.set_always_on_top(self.always_on_top);
                println!("[T] Always on top: {}", self.always_on_top);
            }

            // [M] Toggle maximized
            Key::Character(c) if c == "m" => {
                self.maximized = !self.maximized;
                window.set_maximized(self.maximized);
                println!("[M] Maximized: {}", self.maximized);
            }

            // [C] Toggle cursor visibility
            Key::Character(c) if c == "c" => {
                self.cursor_visible = !self.cursor_visible;
                window.set_cursor_visible(self.cursor_visible);
                println!("[C] Cursor visible: {}", self.cursor_visible);
            }

            // [I] Set a dummy window icon (8x8 red square)
            Key::Character(c) if c == "i" => {
                let (icon_w, icon_h) = (8u32, 8u32);
                let mut rgba = Vec::with_capacity((icon_w * icon_h * 4) as usize);
                for _ in 0..icon_w * icon_h {
                    rgba.extend_from_slice(&[220, 50, 50, 255]); // red
                }
                window.set_window_icon(rgba, icon_w, icon_h);
                println!("[I] Window icon set (8x8 red)");
            }

            // [Escape] Exit
            Key::Named(NamedKey::Escape) => {
                println!("[Esc] Exiting");
                ctx.exit();
            }

            _ => {}
        }
    }
}

fn main() -> Result<(), AppError> {
    env_logger::init();

    println!("=== Window Options Demo ===");
    println!();
    println!("Builder options applied at startup:");
    println!("  - PresentMode: Immediate (VSync OFF)");
    println!("  - Transparent: ON (background alpha = 0.5)");
    println!("  - Decorations: OFF (borderless for transparency)");
    println!("  - Always on top: ON (initial)");
    println!("  - Target FPS: 60 (application-level)");
    println!();
    println!("Runtime key controls:");
    println!("  [F] Toggle borderless fullscreen");
    println!("  [G] Toggle exclusive fullscreen");
    println!("  [T] Toggle always-on-top (bg color changes)");
    println!("  [M] Toggle maximized");
    println!("  [C] Toggle cursor visibility");
    println!("  [I] Set window icon (8x8 red square)");
    println!("  [Esc] Exit");
    println!();
    println!("FPS is printed every second and shown as a green bar at the top.");
    println!("The window background is semi-transparent — the desktop should be visible behind it.");
    println!();

    Application::builder()
        .with_delegate(MyApp)
        .with_target_fps(60)
        .with_window(
            WindowBuilder::new()
                .with_title("Window Options Demo")
                .with_surface_size(960, 540)
                .with_present_mode(PresentMode::Immediate)
                .with_transparent(true)
                .with_decorations(false)
                .with_always_on_top(true)
                .with_renderer_2d()
                .with_delegate(MyWindow::new()),
        )
        .run()
}
