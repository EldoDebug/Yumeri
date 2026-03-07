use yumeri_app::*;
use yumeri_ui::prelude::*;

struct CounterApp;

impl AppDelegate for CounterApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

struct Counter {
    count: i32,
}

impl Component for Counter {
    fn view(&self, ctx: &mut ViewCtx) -> Element {
        Column::new()
            .gap(16.0)
            .padding(32.0)
            .align_items(Align::Center)
            .child(
                Text::new(format!("Count: {}", self.count))
                    .font_size(32.0)
                    .color(Color::WHITE),
            )
            .child(
                Row::new()
                    .gap(12.0)
                    .child(
                        Container::new()
                            .padding_symmetric(16.0, 8.0)
                            .background(Color::rgb(0.25, 0.46, 0.85))
                            .corner_radius(6.0)
                            .align_items(Align::Center)
                            .justify_content(Justify::Center)
                            .on_click(ctx.callback(|this: &mut Self, _| {
                                this.count += 1;
                            }))
                            .child(Text::new("Increment").font_size(16.0).color(Color::WHITE)),
                    )
                    .child(
                        Container::new()
                            .padding_symmetric(16.0, 8.0)
                            .background(Color::rgb(0.8, 0.3, 0.3))
                            .corner_radius(6.0)
                            .align_items(Align::Center)
                            .justify_content(Justify::Center)
                            .on_click(ctx.callback(|this: &mut Self, _| {
                                this.count -= 1;
                            }))
                            .child(Text::new("Decrement").font_size(16.0).color(Color::WHITE)),
                    ),
            )
            .child(
                Container::new()
                    .padding_symmetric(16.0, 8.0)
                    .background(Color::rgb(0.3, 0.3, 0.3))
                    .corner_radius(6.0)
                    .align_items(Align::Center)
                    .justify_content(Justify::Center)
                    .on_click(ctx.callback(|this: &mut Self, _| {
                        this.count = 0;
                    }))
                    .child(Text::new("Reset").font_size(16.0).color(Color::WHITE)),
            )
            .into()
    }
}

struct CounterDelegate {
    ui: UiApp<Counter>,
}

impl WindowDelegate for CounterDelegate {
    fn on_ui_setup(&mut self, ctx: &mut UiContext) {
        let size = ctx.surface_size();
        let (scene, gc) = ctx.scene_and_glyph_cache();
        self.ui.setup(scene, size, gc);
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        let (scene, gc) = ctx.ui_scene_and_glyph_cache();
        if let Some(scene) = scene {
            self.ui.tick(scene, gc);
        }
        ctx.request_redraw();
    }

    fn on_mouse_input(&mut self, _ctx: &mut WindowContext, state: ElementState, button: MouseButton) {
        if state == ElementState::Pressed && button == MouseButton::Left {
            let (x, y) = self.ui.tree().cursor_pos();
            self.ui.on_mouse_click(x, y);
        }
    }

    fn on_cursor_moved(&mut self, _ctx: &mut WindowContext, position: PhysicalPosition<f64>) {
        self.ui.on_cursor_moved(position.x as f32, position.y as f32);
    }

    fn on_resized(&mut self, _ctx: &mut WindowContext, size: PhysicalSize<u32>) {
        self.ui.on_resize(size.width, size.height);
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(CounterApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Counter")
                .with_surface_size(800, 600)
                .with_ui_renderer()
                .with_delegate(CounterDelegate {
                    ui: UiApp::new(|| Counter { count: 0 }),
                }),
        )
        .run()
}
