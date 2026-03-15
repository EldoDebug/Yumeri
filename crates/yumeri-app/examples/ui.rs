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
    rect: Option<NodeId>,
    rounded_rect: Option<NodeId>,
    circle: Option<NodeId>,
    toggle: bool,
}

impl MyWindow {
    fn new() -> Self {
        Self {
            rect: None,
            rounded_rect: None,
            circle: None,
            toggle: false,
        }
    }
}

impl WindowDelegate for MyWindow {
    fn on_ui_setup(&mut self, ctx: &mut UiContext) {
        let (w, h) = ctx.surface_size();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let scene = ctx.scene();

        let rect = scene.add(ShapeType::Rect);
        scene.set_position(rect, [cx - 200.0, cy]);
        scene.set_size(rect, [80.0, 60.0]);
        scene.set_color(rect, Color::rgb(0.2, 0.4, 0.8));
        self.rect = Some(rect);

        let rr = scene.add(ShapeType::RoundedRect);
        scene.set_position(rr, [cx, cy]);
        scene.set_size(rr, [100.0, 60.0]);
        scene.set_corner_radius(rr, 16.0);
        scene.set_color(rr, Color::rgb(0.9, 0.3, 0.3));
        self.rounded_rect = Some(rr);

        let circle = scene.add(ShapeType::Circle);
        scene.set_position(circle, [cx + 200.0, cy]);
        scene.set_size(circle, [50.0, 50.0]);
        scene.set_color(circle, Color::rgba(0.1, 0.8, 0.3, 0.8));
        self.circle = Some(circle);
    }

    fn on_input(&mut self, ctx: &mut WindowContext, event: &InputEvent) {
        let InputEvent::Keyboard(kb) = event else { return };
        if !kb.state.is_pressed() {
            return;
        }
        if kb.code == KeyCode::Space {
            self.toggle = !self.toggle;
            if let Some(scene) = ctx.ui_scene() {
                if let Some(rect) = self.rect {
                    let color = if self.toggle {
                        Color::rgb(0.9, 0.7, 0.1)
                    } else {
                        Color::rgb(0.2, 0.4, 0.8)
                    };
                    scene.set_color(rect, color);
                }
            }
        }
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Yumeri UI Renderer")
                .with_surface_size(1280, 720)
                .with_ui_renderer()
                .with_delegate(MyWindow::new()),
        )
        .run()
}
