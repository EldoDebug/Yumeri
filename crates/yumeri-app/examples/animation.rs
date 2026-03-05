use std::time::{Duration, Instant};

use yumeri_animation::prelude::*;
use yumeri_app::*;

const CARD_COLORS: [[f32; 3]; 5] = [
    [0.2, 0.6, 0.9],
    [0.3, 0.8, 0.5],
    [0.9, 0.6, 0.2],
    [0.8, 0.3, 0.6],
    [0.5, 0.4, 0.9],
];

struct MyApp;

impl AppDelegate for MyApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

struct AnimWindow {
    animator: Animator,
    last_tick: Option<Instant>,

    // Bouncing ball
    ball: Option<NodeId>,
    ball_x: Handle<f32>,
    ball_y: Handle<f32>,
    ball_color: Handle<[f32; 4]>,

    // Sliding cards
    cards: Vec<NodeId>,
    card_xs: Vec<Handle<f32>>,
    card_opacities: Vec<Handle<f32>>,

    // Pulsing circle
    pulse: Option<NodeId>,
    pulse_size: Handle<f32>,

    // Rotating orbit dots
    orbit_center: Option<NodeId>,
    orbit_dots: Vec<NodeId>,
    orbit_angles: Vec<Handle<f32>>,

    surface_w: f32,
    surface_h: f32,
}

impl AnimWindow {
    fn new() -> Self {
        let mut animator = Animator::new();

        // === Bouncing ball (Timeline: X slides right while Y bounces) ===
        let ball_x_tween = Tween::new(100.0_f32, 500.0)
            .duration_ms(2000)
            .easing(Easing::EaseInOutCubic)
            .loop_mode(LoopMode::Infinite)
            .direction(Direction::Alternate)
            .build();
        let ball_x = animator.play(ball_x_tween);

        let ball_y_kf = Keyframes::new()
            .stop(0.0, 300.0_f32)
            .stop_with_easing(0.4, 150.0, Easing::EaseOutQuad)
            .stop_with_easing(0.5, 300.0, Easing::EaseInQuad)
            .stop_with_easing(0.7, 220.0, Easing::EaseOutQuad)
            .stop_with_easing(0.8, 300.0, Easing::EaseInQuad)
            .stop_with_easing(0.9, 270.0, Easing::EaseOutQuad)
            .stop(1.0, 300.0)
            .duration_ms(2000)
            .loop_mode(LoopMode::Infinite)
            .build();
        let ball_y = animator.play_keyframes(ball_y_kf);

        let ball_color = animator.play(
            Tween::new([0.95_f32, 0.3, 0.3, 1.0], [0.3, 0.5, 0.95, 1.0])
                .duration_ms(2000)
                .easing(Easing::EaseInOutSine)
                .loop_mode(LoopMode::Infinite)
                .direction(Direction::Alternate)
                .build(),
        );

        // === Sliding cards with stagger ===
        let card_tweens = stagger(
            5,
            StaggerConfig {
                interval: Duration::from_millis(120),
                from: StaggerFrom::First,
                easing: None,
            },
            |_i| {
                Tween::new(-300.0_f32, 0.0)
                    .duration_ms(800)
                    .easing(Easing::EaseOutBack)
                    .build()
            },
        );
        let card_xs: Vec<_> = card_tweens
            .into_iter()
            .map(|t| animator.play(t))
            .collect();

        let opacity_tweens = stagger(
            5,
            StaggerConfig {
                interval: Duration::from_millis(120),
                from: StaggerFrom::First,
                easing: None,
            },
            |_i| {
                Tween::new(0.0_f32, 1.0)
                    .duration_ms(600)
                    .easing(Easing::EaseOutQuad)
                    .build()
            },
        );
        let card_opacities: Vec<_> = opacity_tweens
            .into_iter()
            .map(|t| animator.play(t))
            .collect();

        // === Pulsing circle ===
        let pulse_size = animator.play(
            Tween::new(30.0_f32, 50.0)
                .duration_ms(1000)
                .easing(Easing::EaseInOutSine)
                .loop_mode(LoopMode::Infinite)
                .direction(Direction::Alternate)
                .build(),
        );

        // === Orbiting dots ===
        let orbit_angles: Vec<_> = (0..6)
            .map(|i| {
                let start = i as f32 * 60.0;
                animator.play(
                    Tween::new(start, start + 360.0)
                        .duration_ms(3000)
                        .easing(Easing::Linear)
                        .loop_mode(LoopMode::Infinite)
                        .build(),
                )
            })
            .collect();

        Self {
            animator,
            last_tick: None,
            ball: None,
            ball_x,
            ball_y,
            ball_color,
            cards: Vec::new(),
            card_xs,
            card_opacities,
            pulse: None,
            pulse_size,
            orbit_center: None,
            orbit_dots: Vec::new(),
            orbit_angles,
            surface_w: 1280.0,
            surface_h: 720.0,
        }
    }
}

impl WindowDelegate for AnimWindow {
    fn on_ui_setup(&mut self, ctx: &mut UiContext) {
        let (w, h) = ctx.surface_size();
        self.surface_w = w as f32;
        self.surface_h = h as f32;
        let scene = ctx.scene();

        // Ball
        let ball = scene.add(ShapeType::Circle);
        scene.set_size(ball, [25.0, 25.0]);
        scene.set_color(ball, Color::rgb(0.95, 0.3, 0.3));
        scene.set_z_index(ball, 10);
        self.ball = Some(ball);

        // Cards (rounded rects stacked vertically on the right side)
        for (i, c) in CARD_COLORS.iter().enumerate() {
            let color = Color::rgba(c[0], c[1], c[2], 1.0);
            let card = scene.add(ShapeType::RoundedRect);
            let base_x = self.surface_w - 200.0;
            let base_y = 120.0 + i as f32 * 85.0;
            scene.set_position(card, [base_x, base_y]);
            scene.set_size(card, [140.0, 35.0]);
            scene.set_corner_radius(card, 12.0);
            scene.set_color(card, color);
            self.cards.push(card);
        }

        // Pulsing circle (center-bottom area)
        let pulse = scene.add(ShapeType::Circle);
        scene.set_position(pulse, [self.surface_w / 2.0, self.surface_h - 120.0]);
        scene.set_size(pulse, [30.0, 30.0]);
        scene.set_color(pulse, Color::rgba(0.9, 0.8, 0.2, 0.7));
        scene.set_z_index(pulse, 5);
        self.pulse = Some(pulse);

        // Orbit center
        let center = scene.add(ShapeType::Circle);
        let cx = self.surface_w / 2.0;
        let cy = self.surface_h / 2.0 - 30.0;
        scene.set_position(center, [cx, cy]);
        scene.set_size(center, [12.0, 12.0]);
        scene.set_color(center, Color::rgba(1.0, 1.0, 1.0, 0.5));
        self.orbit_center = Some(center);

        // Orbit dots
        let dot_colors = [
            Color::rgba(1.0, 0.4, 0.4, 0.9),
            Color::rgba(0.4, 1.0, 0.4, 0.9),
            Color::rgba(0.4, 0.4, 1.0, 0.9),
            Color::rgba(1.0, 1.0, 0.4, 0.9),
            Color::rgba(1.0, 0.4, 1.0, 0.9),
            Color::rgba(0.4, 1.0, 1.0, 0.9),
        ];
        for &color in &dot_colors {
            let dot = scene.add(ShapeType::Circle);
            scene.set_size(dot, [8.0, 8.0]);
            scene.set_color(dot, color);
            scene.set_z_index(dot, 3);
            self.orbit_dots.push(dot);
        }
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        let now = Instant::now();
        let dt = self
            .last_tick
            .map(|last| now.duration_since(last))
            .unwrap_or(Duration::ZERO);
        self.last_tick = Some(now);

        self.animator.update(dt);
        // Drain events to prevent unbounded growth (infinite-loop anims emit every frame)
        let _ = self.animator.drain_events().count();

        if let Some(scene) = ctx.ui_scene() {
            // Update ball
            if let Some(ball) = self.ball {
                let x = self.animator.get(self.ball_x);
                let y = self.animator.get(self.ball_y);
                let rgba = self.animator.get(self.ball_color);
                scene.set_position(ball, [x, y]);
                scene.set_color(ball, Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3]));
            }

            // Update cards
            let base_x = self.surface_w - 200.0;
            for (i, &card) in self.cards.iter().enumerate() {
                let offset_x = self.animator.get(self.card_xs[i]);
                let opacity = self.animator.get(self.card_opacities[i]);
                scene.set_position(card, [base_x + offset_x, 120.0 + i as f32 * 85.0]);
                let c = CARD_COLORS[i];
                scene.set_color(card, Color::rgba(c[0], c[1], c[2], opacity));
            }

            // Update pulse
            if let Some(pulse) = self.pulse {
                let size = self.animator.get(self.pulse_size);
                scene.set_size(pulse, [size, size]);
            }

            // Update orbit dots
            let cx = self.surface_w / 2.0;
            let cy = self.surface_h / 2.0 - 30.0;
            let orbit_radius = 80.0;
            for (i, &dot) in self.orbit_dots.iter().enumerate() {
                let angle_deg = self.animator.get(self.orbit_angles[i]);
                let angle_rad = angle_deg.to_radians();
                let x = cx + orbit_radius * angle_rad.cos();
                let y = cy + orbit_radius * angle_rad.sin();
                scene.set_position(dot, [x, y]);
            }
        }

        ctx.request_redraw();
    }

    fn on_key_input(&mut self, ctx: &mut WindowContext, event: &KeyEvent, is_pressed: bool) {
        if !is_pressed {
            return;
        }
        if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
            ctx.exit();
        }
    }
}

fn main() -> Result<(), AppError> {
    Application::builder()
        .with_delegate(MyApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Yumeri Animation Demo")
                .with_surface_size(1280, 720)
                .with_ui_renderer()
                .with_delegate(AnimWindow::new()),
        )
        .run()
}
