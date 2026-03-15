use yumeri_app::*;

struct MyApp;

impl AppDelegate for MyApp {
    fn on_start(&mut self, ctx: &mut AppContext) {
        println!("App started with {} window(s)", ctx.windows().count());

        ctx.create_window(
            WindowBuilder::new()
                .with_title("Sub Window")
                .with_surface_size(400, 300)
                .with_delegate(SubWindow),
        );
    }

    fn on_window_created(&mut self, _ctx: &mut AppContext, window_id: WindowId) {
        println!("Window created: {window_id:?}");
    }

    fn on_window_destroyed(&mut self, ctx: &mut AppContext, window_id: WindowId) {
        println!("Window destroyed: {window_id:?}");
        if ctx.windows().count() == 0 {
            println!("All windows closed, exiting");
            ctx.exit();
        }
    }

    fn on_stop(&mut self, _ctx: &mut AppContext) {
        println!("App stopped");
    }
}

struct MainWindow;

impl WindowDelegate for MainWindow {
    fn on_redraw_requested(&mut self, _ctx: &mut WindowContext) {}

    fn on_input(&mut self, _ctx: &mut WindowContext, event: &InputEvent) {
        if let InputEvent::Keyboard(kb) = event {
            if kb.state.is_pressed() {
                println!("[Main] Key: {:?}", kb.key);
            }
        }
    }
}

struct SubWindow;

impl WindowDelegate for SubWindow {
    fn on_redraw_requested(&mut self, _ctx: &mut WindowContext) {}

    fn on_close_requested(&mut self, _ctx: &mut WindowContext) -> CloseResponse {
        println!("[Sub] Close requested");
        CloseResponse::Close
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Hello Yumeri")
                .with_surface_size(1280, 720)
                .with_delegate(MainWindow),
        )
        .run()
}
