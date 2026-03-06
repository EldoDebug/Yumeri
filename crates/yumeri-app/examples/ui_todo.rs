use yumeri_app::*;
use yumeri_ui::prelude::*;

struct TodoApp;

impl AppDelegate for TodoApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

#[derive(Clone)]
struct TodoItem {
    text: String,
    done: bool,
}

struct TodoList {
    items: Vec<TodoItem>,
    next_id: usize,
}

impl Component for TodoList {
    fn view(&self, ctx: &mut ViewCtx) -> Element {
        let mut col = Column::new()
            .gap(8.0)
            .padding(24.0)
            .width(Dimension::Percent(1.0))
            .child(
                Text::new("Todo List")
                    .font_size(28.0)
                    .color(Color::WHITE),
            )
            .child(
                Button::new("Add Item")
                    .on_click(ctx.callback(|this: &mut Self, _| {
                        this.items.push(TodoItem {
                            text: format!("Task #{}", this.next_id),
                            done: false,
                        });
                        this.next_id += 1;
                    })),
            );

        for (i, item) in self.items.iter().enumerate() {
            let bg = if item.done {
                Color::rgb(0.15, 0.3, 0.15)
            } else {
                Color::rgb(0.2, 0.2, 0.24)
            };

            col = col.child(
                Row::new()
                    .gap(8.0)
                    .align_items(Align::Center)
                    .child(
                        Checkbox::new(item.done)
                            .on_toggle(ctx.callback(move |this: &mut Self, _| {
                                if let Some(item) = this.items.get_mut(i) {
                                    item.done = !item.done;
                                }
                            })),
                    )
                    .child(
                        Text::new(&item.text)
                            .font_size(16.0)
                            .color(if item.done {
                                Color::rgb(0.5, 0.5, 0.5)
                            } else {
                                Color::WHITE
                            }),
                    )
                    .child(
                        Button::new("Delete")
                            .on_click(ctx.callback(move |this: &mut Self, _| {
                                if i < this.items.len() {
                                    this.items.remove(i);
                                }
                            }))
                            .background(Color::rgb(0.7, 0.2, 0.2))
                            .font_size(12.0),
                    )
                    .background(bg)
                    .corner_radius(4.0)
                    .padding(8.0),
            );
        }

        let total = self.items.len();
        let done = self.items.iter().filter(|i| i.done).count();
        col = col.child(
            Text::new(format!("{done}/{total} completed"))
                .font_size(14.0)
                .color(Color::rgb(0.6, 0.6, 0.65)),
        );

        col.into()
    }
}

struct TodoDelegate {
    ui: UiApp<TodoList>,
}

impl WindowDelegate for TodoDelegate {
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
        .with_delegate(TodoApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Todo List")
                .with_surface_size(600, 500)
                .with_ui_renderer()
                .with_delegate(TodoDelegate {
                    ui: UiApp::new(|| TodoList {
                        items: vec![
                            TodoItem { text: "Learn Rust".into(), done: true },
                            TodoItem { text: "Build UI framework".into(), done: false },
                            TodoItem { text: "Write tests".into(), done: false },
                        ],
                        next_id: 3,
                    }),
                }),
        )
        .run()
}
