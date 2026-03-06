use yumeri_app::*;
use yumeri_video::{Video, VideoPlayer};

const DEFAULT_VIDEO_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test.mp4");

struct MyApp;

impl AppDelegate for MyApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

struct VideoWindow {
    video_path: String,
    texture_id: Option<TextureId>,
}

impl WindowDelegate for VideoWindow {
    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        if self.texture_id.is_none() {
            let vk_info = ctx.vulkan_device_info();
            let player = VideoPlayer::with_vulkan(vk_info).expect("failed to create player");
            let handle = player.play(&self.video_path).expect("failed to play video");
            match ctx.create_video_texture(handle) {
                Ok(id) => self.texture_id = Some(id),
                Err(e) => {
                    eprintln!("Failed to create video texture: {e}");
                    return;
                }
            }
        }

        ctx.update_video_textures();

        if let Some(tex_id) = self.texture_id {
            let (w, h) = ctx.surface_size();
            let tex = Texture::new(tex_id);

            // Draw video filling the entire window
            ctx.draw_rect(Rect {
                position: [w as f32 / 2.0, h as f32 / 2.0],
                size: [w as f32 / 2.0, h as f32 / 2.0],
                color: Color::WHITE,
                texture: Some(tex),
            });
        }
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        ctx.request_redraw();
    }
}

fn main() -> Result<(), AppError> {
    // Default to showing video debug info if RUST_LOG is not set
    if std::env::var("RUST_LOG").is_err() {
        // Safety: called before any threads are spawned
        unsafe {
            std::env::set_var(
                "RUST_LOG",
                "yumeri_video=debug,yumeri_renderer::video=debug,warn",
            );
        }
    }
    env_logger::init();

    let video_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_VIDEO_PATH.to_string());

    let info = Video::probe(&video_path).expect("failed to probe video");
    println!(
        "Video: {}x{}, {:.2} fps, {:.2}s, codec: {:?}, audio: {}",
        info.width(),
        info.height(),
        info.frame_rate(),
        info.duration_secs(),
        info.codec(),
        info.has_audio(),
    );

    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Yumeri Video Player")
                .with_surface_size(info.width().max(640), info.height().max(360))
                .with_renderer_2d()
                .with_delegate(VideoWindow {
                    video_path: video_path,
                    texture_id: None,
                }),
        )
        .run()
}
