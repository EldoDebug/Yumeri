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
    font: Font,
}

impl MyWindow {
    fn new() -> Self {
        Self {
            font: Font::new(),
        }
    }
}

impl WindowDelegate for MyWindow {
    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        let (w, _h) = ctx.surface_size();

        // Default style (16px, white)
        let default_style = TextStyle::default();
        ctx.draw_text(&mut self.font, "Hello, Yumeri!", [40.0, 40.0], &default_style);

        // Large title
        let title_style = TextStyle {
            font_size: 48.0,
            line_height: 56.0,
            color: Color::rgb(0.3, 0.6, 1.0),
            ..Default::default()
        };
        ctx.draw_text(&mut self.font, "Text Rendering", [40.0, 80.0], &title_style);

        // Colored text
        let red_style = TextStyle {
            font_size: 24.0,
            line_height: 32.0,
            color: Color::rgb(1.0, 0.3, 0.3),
            ..Default::default()
        };
        ctx.draw_text(&mut self.font, "Red text with 24px font size", [40.0, 160.0], &red_style);

        let green_style = TextStyle {
            font_size: 24.0,
            line_height: 32.0,
            color: Color::rgb(0.2, 0.9, 0.4),
            ..Default::default()
        };
        ctx.draw_text(&mut self.font, "Green text", [40.0, 200.0], &green_style);

        // Word-wrapped text
        let wrap_style = TextStyle {
            font_size: 18.0,
            line_height: 26.0,
            color: Color::rgb(0.9, 0.9, 0.9),
            max_width: Some(w as f32 - 80.0),
            wrap: WrapMode::Word,
            ..Default::default()
        };
        ctx.draw_text(
            &mut self.font,
            "This is a longer paragraph that demonstrates word wrapping. \
             When the text exceeds the max_width, it automatically wraps to the next line. \
             The glyph atlas caches rasterized glyphs for efficient GPU rendering.",
            [40.0, 260.0],
            &wrap_style,
        );

        // Japanese text
        let jp_style = TextStyle {
            font_size: 28.0,
            line_height: 36.0,
            color: Color::rgb(1.0, 0.8, 0.2),
            ..Default::default()
        };
        ctx.draw_text(&mut self.font, "日本語テキスト描画テスト", [40.0, 400.0], &jp_style);

        // Mixed text with emoji
        let emoji_style = TextStyle {
            font_size: 24.0,
            line_height: 32.0,
            color: Color::WHITE,
            ..Default::default()
        };
        ctx.draw_text(
            &mut self.font,
            "Emoji: 🎨🚀✨ Mixed: Hello 世界!",
            [40.0, 460.0],
            &emoji_style,
        );

        // Small text
        let small_style = TextStyle {
            font_size: 12.0,
            line_height: 16.0,
            color: Color::rgba(0.7, 0.7, 0.7, 0.8),
            ..Default::default()
        };
        ctx.draw_text(
            &mut self.font,
            "Small 12px text for captions and labels",
            [40.0, 520.0],
            &small_style,
        );
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
                .with_title("Yumeri Text Rendering")
                .with_surface_size(1280, 720)
                .with_renderer_2d()
                .with_delegate(MyWindow::new()),
        )
        .run()
}
