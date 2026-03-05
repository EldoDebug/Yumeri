use yumeri_app::*;

struct MyApp;

impl AppDelegate for MyApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

struct MyWindow;

impl WindowDelegate for MyWindow {
    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        let (w, h) = ctx.surface_size();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;

        ctx.draw_rect(Rect {
            position: [cx - 200.0, cy],
            size: [80.0, 60.0],
            color: Color::rgb(0.2, 0.4, 0.8),
            texture: None,
        });

        ctx.draw_rounded_rect(RoundedRect {
            position: [cx, cy],
            size: [100.0, 60.0],
            corner_radius: 16.0,
            color: Color::rgb(0.9, 0.3, 0.3),
            texture: None,
        });

        ctx.draw_circle(Circle {
            position: [cx + 200.0, cy],
            radius: 50.0,
            color: Color::rgba(0.1, 0.8, 0.3, 0.8),
            texture: None,
        });
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        ctx.request_redraw();
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Yumeri 2D Renderer")
                .with_surface_size(1280, 720)
                .with_renderer_2d()
                .with_delegate(MyWindow),
        )
        .run()
}
