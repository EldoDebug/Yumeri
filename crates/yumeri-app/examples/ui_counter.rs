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
                        Button::new("Increment")
                            .on_click(ctx.callback(|this: &mut Self, _| {
                                this.count += 1;
                            })),
                    )
                    .child(
                        Button::new("Decrement")
                            .on_click(ctx.callback(|this: &mut Self, _| {
                                this.count -= 1;
                            }))
                            .background(Color::rgb(0.8, 0.3, 0.3)),
                    ),
            )
            .child(
                Button::new("Reset")
                    .on_click(ctx.callback(|this: &mut Self, _| {
                        this.count = 0;
                    }))
                    .background(Color::rgb(0.3, 0.3, 0.3)),
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
        let scene = ctx.scene();
        self.ui.setup(scene, size);
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        if let Some(scene) = ctx.ui_scene() {
            self.ui.tick(scene);
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
