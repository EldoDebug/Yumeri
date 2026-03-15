use std::sync::{Arc, Mutex};
use std::time::Duration;

use yumeri_animation::prelude::*;
use yumeri_app::*;
use yumeri_components::Checkbox;
use yumeri_template::TemplateRegistry;
use yumeri_ui::prelude::*;

use yumeri_animation::playback::Direction as AnimDirection;

struct ShowcaseApp;

impl AppDelegate for ShowcaseApp {
    fn on_window_destroyed(&mut self, ctx: &mut AppContext, _window_id: WindowId) {
        if ctx.windows().count() == 0 {
            ctx.exit();
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ThemeKind {
    Yumeri,
    Dark,
    Warm,
}

// Shared animation handles between delegate and component
struct AnimHandles {
    // Basic transforms
    spin: Option<Handle<f32>>,
    pulse: Option<Handle<[f32; 2]>>,
    float: Option<Handle<[f32; 2]>>,
    color: Option<Handle<[f32; 4]>>,

    // Stagger wave (6 bars)
    wave_ys: Vec<Handle<[f32; 2]>>,

    // Elastic bounce
    bounce_scale: Option<Handle<[f32; 2]>>,

    // Combined transforms on one shape
    combined_rotation: Option<Handle<f32>>,
    combined_scale: Option<Handle<[f32; 2]>>,
    combined_translate: Option<Handle<[f32; 2]>>,

    // Timeline choreography (3 shapes)
    choreo_slide: Option<Handle<[f32; 2]>>,
    choreo_scale: Option<Handle<[f32; 2]>>,
    choreo_spin: Option<Handle<f32>>,
}

type SharedHandles = Arc<Mutex<AnimHandles>>;

const WAVE_COLORS: [[f32; 3]; 6] = [
    [1.0, 0.40, 0.56],  // pink
    [1.0, 0.55, 0.78],  // light pink
    [0.17, 0.62, 0.85],  // cyan
    [0.24, 0.89, 0.93],  // light cyan
    [0.36, 0.93, 0.79],  // mint
    [1.0, 0.84, 0.40],  // gold
];

type SharedTheme = Arc<Mutex<ThemeKind>>;

struct Showcase {
    theme: ThemeKind,
    shared_theme: SharedTheme,
    handles: SharedHandles,
}

impl Component for Showcase {
    fn view(&self, ctx: &mut ViewCtx) -> Element {
        let h = self.handles.lock().unwrap();
        let a = ctx.animator();

        // Basic transforms
        let spin = h.spin.map(|x| a.get(x)).unwrap_or(0.0);
        let pulse_scale = h.pulse.map(|x| a.get(x)).unwrap_or([1.0, 1.0]);
        let float_translate = h.float.map(|x| a.get(x)).unwrap_or([0.0, 0.0]);
        let color_rgba = h.color.map(|x| a.get(x)).unwrap_or([0.17, 0.62, 0.85, 1.0]);

        // Stagger wave
        let wave_ys: Vec<[f32; 2]> = h.wave_ys.iter().map(|&x| a.get(x)).collect();

        // Elastic bounce
        let bounce = h.bounce_scale.map(|x| a.get(x)).unwrap_or([1.0, 1.0]);

        // Combined
        let comb_rot = h.combined_rotation.map(|x| a.get(x)).unwrap_or(0.0);
        let comb_scale = h.combined_scale.map(|x| a.get(x)).unwrap_or([1.0, 1.0]);
        let comb_trans = h.combined_translate.map(|x| a.get(x)).unwrap_or([0.0, 0.0]);

        // Choreography
        let choreo_sl = h.choreo_slide.map(|x| a.get(x)).unwrap_or([-80.0, 0.0]);
        let choreo_sc = h.choreo_scale.map(|x| a.get(x)).unwrap_or([0.0, 0.0]);
        let choreo_sp = h.choreo_spin.map(|x| a.get(x)).unwrap_or(0.0);

        drop(h);

        Column::new()
            .padding(32.0)
            .gap(24.0)
            .child(section_title("Template Showcase"))
            // Checkboxes
            .child(section("Checkboxes", Column::new()
                .gap(12.0)
                .child(Element::component(|| Checkbox::new(false).label("Unchecked")))
                .child(Element::component(|| Checkbox::new(true).label("Pre-checked")))
                .child(Element::component(|| Checkbox::new(false)))
            ))
            // Theme Switching
            .child(section("Theme Switching", Row::new()
                .gap(12.0)
                .child(theme_button(ctx, "Yumeri", ThemeKind::Yumeri, self.theme))
                .child(theme_button(ctx, "Dark", ThemeKind::Dark, self.theme))
                .child(theme_button(ctx, "Warm", ThemeKind::Warm, self.theme))
            ))
            // Shape Primitives
            .child(section("Shape Primitives", Row::new()
                .gap(16.0)
                .align_items(Align::Center)
                .child(RectWidget::new().width(60.0).height(60.0).background(Color::rgb(0.17, 0.62, 0.85)))
                .child(RoundedRectWidget::new().width(60.0).height(60.0).corner_radius(12.0).background(Color::rgb(1.0, 0.40, 0.56)))
                .child(CircleWidget::new().width(60.0).height(60.0).background(Color::rgb(0.36, 0.93, 0.79)))
                .child(EllipseWidget::new().width(80.0).height(50.0).background(Color::rgb(1.0, 0.84, 0.40)))
            ))
            // Animated Transforms
            .child(section("Animated Transforms", Row::new()
                .gap(32.0)
                .align_items(Align::Center)
                .child(labeled_demo("Rotation", {
                    let mut r = RectWidget::new().width(50.0).height(50.0).background(Color::rgb(0.24, 0.89, 0.93));
                    r.style.rotation = spin;
                    r
                }))
                .child(labeled_demo("Scale", {
                    let mut r = RoundedRectWidget::new().width(50.0).height(50.0).corner_radius(8.0).background(Color::rgb(1.0, 0.55, 0.78));
                    r.style.scale = pulse_scale;
                    r
                }))
                .child(labeled_demo("Translate", {
                    let mut c = CircleWidget::new().width(50.0).height(50.0).background(Color::rgb(0.42, 0.72, 1.0));
                    c.style.translate = float_translate;
                    c
                }))
                .child(labeled_demo("Color", {
                    RectWidget::new().width(50.0).height(50.0).background(Color::rgba(color_rgba[0], color_rgba[1], color_rgba[2], color_rgba[3]))
                }))
            ))
            // --- Complex Animations ---
            // Stagger wave
            .child(section("Stagger Wave", {
                let mut row = Row::new().gap(8.0).align_items(Align::Center);
                for (i, translate) in wave_ys.iter().enumerate() {
                    let c = WAVE_COLORS[i % WAVE_COLORS.len()];
                    let mut bar = RoundedRectWidget::new()
                        .width(24.0)
                        .height(60.0)
                        .corner_radius(6.0)
                        .background(Color::rgb(c[0], c[1], c[2]));
                    bar.style.translate = *translate;
                    row = row.child(bar);
                }
                row
            }))
            // Elastic bounce
            .child(section("Elastic Bounce", Row::new()
                .gap(32.0)
                .align_items(Align::Center)
                .child(labeled_demo("Keyframes", {
                    let mut c = CircleWidget::new().width(60.0).height(60.0).background(Color::rgb(1.0, 0.40, 0.56));
                    c.style.scale = bounce;
                    c
                }))
            ))
            // Combined transforms
            .child(section("Combined Transforms", Row::new()
                .gap(32.0)
                .align_items(Align::Center)
                .child(labeled_demo("Rot + Scale + Move", {
                    let mut r = RoundedRectWidget::new()
                        .width(50.0)
                        .height(50.0)
                        .corner_radius(10.0)
                        .background(Color::rgb(0.42, 0.72, 1.0));
                    r.style.rotation = comb_rot;
                    r.style.scale = comb_scale;
                    r.style.translate = comb_trans;
                    r
                }))
            ))
            // Timeline choreography
            .child(section("Timeline Choreography", Row::new()
                .gap(24.0)
                .align_items(Align::Center)
                .child(labeled_demo("Slide In", {
                    let mut r = RectWidget::new().width(50.0).height(50.0).background(Color::rgb(0.17, 0.62, 0.85));
                    r.style.translate = choreo_sl;
                    r
                }))
                .child(labeled_demo("Pop Up", {
                    let mut r = CircleWidget::new().width(50.0).height(50.0).background(Color::rgb(1.0, 0.55, 0.78));
                    r.style.scale = choreo_sc;
                    r
                }))
                .child(labeled_demo("Spin In", {
                    let mut r = RoundedRectWidget::new().width(50.0).height(50.0).corner_radius(8.0).background(Color::rgb(0.36, 0.93, 0.79));
                    r.style.rotation = choreo_sp;
                    r
                }))
            ))
            .into()
    }
}

fn section_title(title: &str) -> Text {
    Text::new(title).font_size(28.0).color(Color::WHITE)
}

fn section(label: &str, content: impl Into<Element>) -> Container {
    Container::new()
        .gap(8.0)
        .child(Text::new(label).font_size(18.0).color(Color::rgb(0.7, 0.7, 0.8)))
        .child(
            Container::new()
                .padding(16.0)
                .corner_radius(12.0)
                .background(Color::rgba(1.0, 1.0, 1.0, 0.05))
                .child(content)
        )
}

fn labeled_demo(label: &str, widget: impl Into<Element>) -> Column {
    Column::new()
        .gap(8.0)
        .align_items(Align::Center)
        .child(widget)
        .child(Text::new(label).font_size(12.0).color(Color::rgb(0.6, 0.6, 0.7)))
}

fn theme_button(ctx: &mut ViewCtx, label: &str, kind: ThemeKind, current: ThemeKind) -> Container {
    let is_active = kind == current;
    let bg = if is_active {
        Color::rgb(0.17, 0.62, 0.85)
    } else {
        Color::rgb(0.25, 0.25, 0.30)
    };
    Container::new()
        .padding_symmetric(16.0, 8.0)
        .background(bg)
        .corner_radius(8.0)
        .align_items(Align::Center)
        .justify_content(Justify::Center)
        .on_click(ctx.callback(move |this: &mut Showcase, _| {
            this.theme = kind;
            *this.shared_theme.lock().unwrap() = kind;
        }))
        .child(Text::new(label).font_size(14.0).color(Color::WHITE))
}

fn create_animations(animator: &mut Animator) -> AnimHandles {
    // === Basic transforms ===
    let spin = Some(animator.play(
        Tween::new(0.0_f32, std::f32::consts::TAU)
            .duration_ms(3000)
            .easing(Easing::Linear)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    let pulse = Some(animator.play(
        Tween::new([1.0_f32, 1.0], [1.3, 1.3])
            .duration_ms(800)
            .easing(Easing::EaseInOutSine)
            .loop_mode(LoopMode::Infinite)
            .direction(AnimDirection::Alternate)
            .build(),
    ));

    let float = Some(animator.play(
        Tween::new([0.0_f32, 0.0], [0.0, -15.0])
            .duration_ms(1200)
            .easing(Easing::EaseInOutSine)
            .loop_mode(LoopMode::Infinite)
            .direction(AnimDirection::Alternate)
            .build(),
    ));

    let color = Some(animator.play(
        Tween::new([0.17_f32, 0.62, 0.85, 1.0], [1.0, 0.40, 0.56, 1.0])
            .duration_ms(2000)
            .easing(Easing::EaseInOutCubic)
            .loop_mode(LoopMode::Infinite)
            .direction(AnimDirection::Alternate)
            .build(),
    ));

    // === Stagger wave (6 bars, bounce up/down from center) ===
    let wave_tweens = stagger(
        6,
        StaggerConfig {
            interval: Duration::from_millis(80),
            from: StaggerFrom::Center,
            easing: None,
        },
        |_| {
            Tween::new([0.0_f32, 0.0], [0.0, -20.0])
                .duration_ms(500)
                .easing(Easing::EaseInOutSine)
                .loop_mode(LoopMode::Infinite)
                .direction(AnimDirection::Alternate)
                .build()
        },
    );
    let wave_ys: Vec<_> = wave_tweens.into_iter().map(|t| animator.play(t)).collect();

    // === Elastic bounce (keyframes: overshoot → compress → settle) ===
    let bounce_scale = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, [1.0_f32, 1.0])
            .stop_with_easing(0.15, [1.4, 0.7], Easing::EaseOutQuad)
            .stop_with_easing(0.30, [0.8, 1.25], Easing::EaseOutQuad)
            .stop_with_easing(0.45, [1.2, 0.85], Easing::EaseOutQuad)
            .stop_with_easing(0.60, [0.92, 1.1], Easing::EaseOutQuad)
            .stop_with_easing(0.75, [1.05, 0.97], Easing::EaseOutQuad)
            .stop(1.0, [1.0, 1.0])
            .duration_ms(1500)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    // === Combined: rotation + scale + translate on one shape ===
    let combined_rotation = Some(animator.play(
        Tween::new(0.0_f32, std::f32::consts::TAU)
            .duration_ms(4000)
            .easing(Easing::EaseInOutCubic)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    let combined_scale = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, [1.0_f32, 1.0])
            .stop_with_easing(0.25, [1.3, 1.3], Easing::EaseOutBack)
            .stop_with_easing(0.5, [1.0, 1.0], Easing::EaseInOutSine)
            .stop_with_easing(0.75, [0.8, 0.8], Easing::EaseInOutSine)
            .stop(1.0, [1.0, 1.0])
            .duration_ms(4000)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    let combined_translate = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, [0.0_f32, 0.0])
            .stop_with_easing(0.25, [20.0, -10.0], Easing::EaseOutCubic)
            .stop_with_easing(0.5, [0.0, 0.0], Easing::EaseInOutSine)
            .stop_with_easing(0.75, [-20.0, 10.0], Easing::EaseOutCubic)
            .stop(1.0, [0.0, 0.0])
            .duration_ms(4000)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    // === Timeline choreography: slide → pop → spin (sequenced via keyframes) ===
    // Total cycle: 2400ms (600 slide + 500 pop + 800 spin + 500 pause)
    let total = 2400.0;
    let slide_end = 600.0 / total;      // 0.25
    let pop_start = slide_end;           // 0.25
    let pop_end = (600.0 + 500.0) / total; // ~0.458
    let spin_start = pop_end;            // ~0.458
    let spin_end = (600.0 + 500.0 + 800.0) / total; // ~0.792

    // Shape 1: slide in during 0..slide_end, hold rest
    let choreo_slide = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, [-80.0_f32, 0.0])
            .stop_with_easing(slide_end, [0.0, 0.0], Easing::EaseOutBack)
            .stop(spin_end, [0.0, 0.0])
            .stop(1.0, [-80.0, 0.0]) // reset for next loop
            .duration_ms(total as u64)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    // Shape 2: idle until pop_start, pop during pop_start..pop_end, hold rest
    let choreo_scale = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, [0.0_f32, 0.0])
            .stop(pop_start, [0.0, 0.0])
            .stop_with_easing(pop_end, [1.0, 1.0], Easing::EaseOutBack)
            .stop(spin_end, [1.0, 1.0])
            .stop(1.0, [0.0, 0.0]) // reset
            .duration_ms(total as u64)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    // Shape 3: idle until spin_start, spin during spin_start..spin_end, hold rest
    let tau2 = std::f32::consts::TAU * 2.0;
    let choreo_spin = Some(animator.play_keyframes(
        Keyframes::new()
            .stop(0.0, 0.0_f32)
            .stop(spin_start, 0.0)
            .stop_with_easing(spin_end, tau2, Easing::EaseOutCubic)
            .stop(1.0, 0.0) // reset
            .duration_ms(total as u64)
            .loop_mode(LoopMode::Infinite)
            .build(),
    ));

    AnimHandles {
        spin,
        pulse,
        float,
        color,
        wave_ys,
        bounce_scale,
        combined_rotation,
        combined_scale,
        combined_translate,
        choreo_slide,
        choreo_scale,
        choreo_spin,
    }
}

fn apply_theme(reg: &mut TemplateRegistry, theme: ThemeKind) {
    use yumeri_template::TokenValue;
    match theme {
        ThemeKind::Yumeri => {} // default tokens are already applied
        ThemeKind::Dark => {
            reg.set_token("surface", TokenValue::Color(0.12, 0.12, 0.15, 1.0));
            reg.set_token("surface-hover", TokenValue::Color(0.16, 0.16, 0.20, 1.0));
            reg.set_token("surface-variant", TokenValue::Color(0.14, 0.14, 0.17, 1.0));
            reg.set_token("on-surface", TokenValue::Color(0.90, 0.90, 0.93, 1.0));
            reg.set_token("on-surface-variant", TokenValue::Color(0.60, 0.62, 0.68, 1.0));
            reg.set_token("border", TokenValue::Color(0.25, 0.25, 0.30, 1.0));
            reg.set_token("border-strong", TokenValue::Color(0.35, 0.35, 0.40, 1.0));
            reg.set_token("text", TokenValue::Color(0.90, 0.90, 0.93, 1.0));
            reg.set_token("text-heading", TokenValue::Color(0.96, 0.96, 0.98, 1.0));
            reg.set_token("text-secondary", TokenValue::Color(0.60, 0.62, 0.68, 1.0));
            reg.set_token("text-disabled", TokenValue::Color(0.40, 0.42, 0.48, 1.0));
        }
        ThemeKind::Warm => {
            reg.set_token("primary", TokenValue::Color(0.93, 0.55, 0.25, 1.0));
            reg.set_token("primary-hover", TokenValue::Color(0.95, 0.65, 0.35, 1.0));
            reg.set_token("primary-light", TokenValue::Color(1.0, 0.92, 0.82, 1.0));
            reg.set_token("surface", TokenValue::Color(1.0, 0.99, 0.97, 1.0));
            reg.set_token("surface-hover", TokenValue::Color(0.99, 0.97, 0.93, 1.0));
            reg.set_token("border", TokenValue::Color(0.88, 0.84, 0.78, 1.0));
            reg.set_token("border-strong", TokenValue::Color(0.78, 0.74, 0.68, 1.0));
        }
    }
}

fn create_registry(theme: ThemeKind) -> TemplateRegistry {
    let mut reg = TemplateRegistry::new();
    let templates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../yumeri-components/templates");
    if templates_dir.exists() {
        let _ = reg.load_dir(&templates_dir);
    }
    apply_theme(&mut reg, theme);
    reg
}

struct ShowcaseDelegate {
    ui: UiApp<Showcase>,
    handles: SharedHandles,
    theme: SharedTheme,
    last_theme: ThemeKind,
    animations_started: bool,
}

impl WindowDelegate for ShowcaseDelegate {
    fn on_ui_setup(&mut self, ctx: &mut UiContext) {
        self.ui.set_template_provider(create_registry(self.last_theme));
        let size = ctx.surface_size();
        let (scene, gc) = ctx.scene_and_glyph_cache();
        self.ui.setup(scene, size, gc);
    }

    fn on_redraw_requested(&mut self, ctx: &mut WindowContext) {
        let current_theme = *self.theme.lock().unwrap();
        if current_theme != self.last_theme {
            self.ui.set_template_provider(create_registry(current_theme));
            self.last_theme = current_theme;
            self.ui.tree_mut().request_rebuild();
        }

        if !self.animations_started {
            let new_handles = create_animations(self.ui.tree_mut().animator());
            *self.handles.lock().unwrap() = new_handles;
            self.animations_started = true;
        }

        let (scene, gc) = ctx.ui_scene_and_glyph_cache();
        if let Some(scene) = scene {
            self.ui.tick(scene, gc);
        }
        ctx.request_redraw();
    }

    fn on_input(&mut self, _ctx: &mut WindowContext, event: &InputEvent) {
        self.ui.on_input(event);
    }

    fn on_resized(&mut self, _ctx: &mut WindowContext, size: PhysicalSize<u32>) {
        self.ui.on_resize(size.width, size.height);
    }
}

fn main() -> Result<(), AppError> {
    let handles: SharedHandles = Arc::new(Mutex::new(AnimHandles {
        spin: None,
        pulse: None,
        float: None,
        color: None,
        wave_ys: Vec::new(),
        bounce_scale: None,
        combined_rotation: None,
        combined_scale: None,
        combined_translate: None,
        choreo_slide: None,
        choreo_scale: None,
        choreo_spin: None,
    }));
    let theme: SharedTheme = Arc::new(Mutex::new(ThemeKind::Yumeri));

    let component_handles = handles.clone();
    let component_theme = theme.clone();

    Application::builder()
        .with_delegate(ShowcaseApp)
        .with_window(
            WindowBuilder::new()
                .with_title("Template Showcase")
                .with_surface_size(1024, 900)
                .with_ui_renderer()
                .with_delegate(ShowcaseDelegate {
                    ui: UiApp::new(move || Showcase {
                        theme: ThemeKind::Yumeri,
                        shared_theme: component_theme.clone(),
                        handles: component_handles.clone(),
                    }),
                    handles,
                    theme,
                    last_theme: ThemeKind::Yumeri,
                    animations_started: false,
                }),
        )
        .run()
}
