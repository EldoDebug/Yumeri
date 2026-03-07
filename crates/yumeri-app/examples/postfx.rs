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
    initialized: bool,
    mask_active: bool,
}

impl MyWindow {
    fn new() -> Self {
        Self {
            initialized: false,
            mask_active: false,
        }
    }
}

/// Generate a circular mask (R8): white inside the circle, black outside.
/// The circle is centered at UV (0.5, 0.5) with a small radius so only
/// the center portion of the screen is affected by the post-effect.
fn create_circle_mask(width: u32, height: u32) -> Vec<u8> {
    let cx = width as f32 / 2.0;
    let cy = height as f32 / 2.0;
    // Use a fixed pixel radius so the circle covers roughly the center shape
    let radius = (width.min(height) as f32) * 0.15;
    let mut data = vec![0u8; (width * height) as usize];
    for y in 0..height {
        for x in 0..width {
            let dx = x as f32 + 0.5 - cx;
            let dy = y as f32 + 0.5 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            // Soft edge over ~4 pixels
            let alpha = ((radius - dist) / 4.0).clamp(0.0, 1.0);
            data[(y * width + x) as usize] = (alpha * 255.0) as u8;
        }
    }
    data
}

impl WindowDelegate for MyWindow {
    fn on_render2d(&mut self, ctx: &mut RenderContext2D) {
        let (w, h) = ctx.surface_size();
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;

        // Colorful background strips so the mask effect is visible everywhere
        let strip_h = h as f32 / 6.0;
        let colors = [
            Color::rgb(0.8, 0.2, 0.2),
            Color::rgb(0.2, 0.7, 0.2),
            Color::rgb(0.2, 0.3, 0.8),
            Color::rgb(0.8, 0.7, 0.1),
            Color::rgb(0.7, 0.2, 0.7),
            Color::rgb(0.1, 0.7, 0.7),
        ];
        for (i, &color) in colors.iter().enumerate() {
            ctx.draw_rect(Rect {
                position: [cx, strip_h * (i as f32 + 0.5)],
                size: [w as f32, strip_h],
                color,
                texture: None,
            });
        }

        // Shapes on top
        ctx.draw_rect(Rect {
            position: [cx - 200.0, cy],
            size: [100.0, 80.0],
            color: Color::rgb(1.0, 1.0, 1.0),
            texture: None,
        });

        ctx.draw_rounded_rect(RoundedRect {
            position: [cx, cy],
            size: [120.0, 80.0],
            corner_radius: 16.0,
            color: Color::rgb(1.0, 0.5, 0.0),
            texture: None,
        });

        ctx.draw_circle(Circle {
            position: [cx + 200.0, cy],
            radius: 50.0,
            color: Color::rgb(0.0, 1.0, 0.5),
            texture: None,
        });
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        if !self.initialized {
            self.initialized = true;
            if let Err(e) = ctx.add_post_effect(Box::new(Grayscale::new())) {
                eprintln!("Failed to add grayscale effect: {e}");
            }
            println!("Controls:");
            println!("  [G]       Toggle grayscale on/off");
            println!("  [M]       Toggle circular mask");
            println!("  [Up/Down] Adjust intensity");
        }
        ctx.request_redraw();
    }

    fn on_key_input(&mut self, ctx: &mut WindowContext, event: &KeyEvent, is_pressed: bool) {
        if !is_pressed {
            return;
        }
        match event.logical_key.as_ref() {
            // [G] Toggle grayscale on/off
            Key::Character("g") => {
                let has_effect = ctx
                    .post_effect_chain()
                    .and_then(|c| c.get::<Grayscale>(Grayscale::NAME))
                    .is_some();

                if has_effect {
                    ctx.remove_post_effect(Grayscale::NAME);
                    println!("Grayscale: OFF");
                } else {
                    if let Err(e) = ctx.add_post_effect(Box::new(Grayscale::new())) {
                        eprintln!("Failed to add grayscale: {e}");
                    }
                    // Restore mask state if it was active
                    if self.mask_active {
                        if let Some(chain) = ctx.post_effect_chain_mut() {
                            if let Some(gs) = chain.get_mut::<Grayscale>(Grayscale::NAME) {
                                gs.set_mask_enabled(true);
                            }
                        }
                    }
                    println!("Grayscale: ON (intensity=1.0)");
                }
            }
            // [M] Toggle circular mask
            Key::Character("m") => {
                self.mask_active = !self.mask_active;
                if self.mask_active {
                    let size = ctx.window().surface_size();
                    let mask_data = create_circle_mask(size.width, size.height);
                    if let Err(e) =
                        ctx.set_post_effect_mask_from_data(size.width, size.height, &mask_data)
                    {
                        eprintln!("Failed to set mask: {e}");
                        self.mask_active = false;
                        return;
                    }
                    if let Some(chain) = ctx.post_effect_chain_mut() {
                        if let Some(gs) = chain.get_mut::<Grayscale>(Grayscale::NAME) {
                            gs.set_mask_enabled(true);
                        }
                    }
                    println!("Mask: ON (circle)");
                } else {
                    ctx.clear_post_effect_mask();
                    if let Some(chain) = ctx.post_effect_chain_mut() {
                        if let Some(gs) = chain.get_mut::<Grayscale>(Grayscale::NAME) {
                            gs.set_mask_enabled(false);
                        }
                    }
                    println!("Mask: OFF");
                }
            }
            // [Up/Down] Adjust intensity
            Key::Named(NamedKey::ArrowUp) => {
                if let Some(chain) = ctx.post_effect_chain_mut() {
                    if let Some(gs) = chain.get_mut::<Grayscale>(Grayscale::NAME) {
                        let new = (gs.intensity() + 0.1).min(1.0);
                        gs.set_intensity(new);
                        println!("Grayscale intensity: {new:.1}");
                    }
                }
            }
            Key::Named(NamedKey::ArrowDown) => {
                if let Some(chain) = ctx.post_effect_chain_mut() {
                    if let Some(gs) = chain.get_mut::<Grayscale>(Grayscale::NAME) {
                        let new = (gs.intensity() - 0.1).max(0.0);
                        gs.set_intensity(new);
                        println!("Grayscale intensity: {new:.1}");
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Yumeri Post-Effects")
                .with_surface_size(1280, 720)
                .with_renderer_2d()
                .with_delegate(MyWindow::new()),
        )
        .run()
}
