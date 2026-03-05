use yumeri_app::*;

const TEXTURE_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test.png");

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

        // load_texture is idempotent — safe to call every frame
        let tex_id = ctx.load_texture(TEXTURE_PATH);
        let tex = Texture::new(tex_id);

        // Plain image in a rect
        ctx.draw_rect(Rect {
            position: [cx - 250.0, cy],
            size: [100.0, 100.0],
            color: Color::WHITE,
            texture: Some(tex),
        });

        // Image in a rounded rect
        ctx.draw_rounded_rect(RoundedRect {
            position: [cx, cy],
            size: [100.0, 100.0],
            corner_radius: 20.0,
            color: Color::WHITE,
            texture: Some(tex),
        });

        // Image in a circle (avatar style)
        ctx.draw_circle(Circle {
            position: [cx + 250.0, cy],
            radius: 80.0,
            color: Color::WHITE,
            texture: Some(tex),
        });

        // Tinted image — reddish overlay
        ctx.draw_rounded_rect(RoundedRect {
            position: [cx - 125.0, cy + 230.0],
            size: [80.0, 80.0],
            corner_radius: 12.0,
            color: Color::rgba(1.0, 0.5, 0.5, 0.9),
            texture: Some(tex),
        });

        // UV sub-region (top-left quarter)
        ctx.draw_rect(Rect {
            position: [cx + 125.0, cy + 230.0],
            size: [80.0, 80.0],
            color: Color::WHITE,
            texture: Some(tex.uv(0.0, 0.0, 0.5, 0.5)),
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
                .with_title("Yumeri Texture Demo")
                .with_surface_size(1280, 720)
                .with_renderer_2d()
                .with_delegate(MyWindow),
        )
        .run()
}
