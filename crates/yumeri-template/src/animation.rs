use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub enum AnimationDef {
    Keyframes(KeyframesSpec),
    Timeline(TimelineSpec),
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyframesSpec {
    pub property: AnimatableProperty,
    pub frames: Vec<KeyframePoint>,
    pub duration_ms: u64,
    pub easing: EasingKind,
    pub loop_mode: LoopMode,
    pub direction: AnimDirection,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KeyframePoint {
    pub at: f32,
    pub value: AnimValue,
}

#[derive(Clone, Debug, Deserialize)]
pub enum AnimValue {
    Float(f32),
    Color(f32, f32, f32, f32),
    Vec2([f32; 2]),
}

#[derive(Clone, Debug, Deserialize)]
pub enum AnimatableProperty {
    Opacity,
    BackgroundColor,
    Width,
    Height,
    CornerRadius,
    Translate,
    Scale,
    Rotation,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimelineSpec {
    pub tracks: Vec<TimelineTrack>,
    pub duration_ms: u64,
    pub loop_mode: LoopMode,
}

#[derive(Clone, Debug, Deserialize)]
pub struct TimelineTrack {
    pub target: String,
    pub animation: KeyframesSpec,
    pub offset: TimeOffset,
}

#[derive(Clone, Debug, Deserialize)]
pub enum TimeOffset {
    Start,
    At(u64),
    AfterPrevious,
    AfterPreviousWithDelay(u64),
    WithPrevious,
}

#[derive(Clone, Debug, Deserialize)]
pub enum LoopMode {
    None,
    Forever,
    Count(u32),
}

#[derive(Clone, Debug, Deserialize)]
pub enum AnimDirection {
    Normal,
    Reverse,
}

#[derive(Clone, Debug, Deserialize)]
pub enum EasingKind {
    Linear,
    EaseInSine, EaseOutSine, EaseInOutSine,
    EaseInQuad, EaseOutQuad, EaseInOutQuad,
    EaseInCubic, EaseOutCubic, EaseInOutCubic,
    EaseInQuart, EaseOutQuart, EaseInOutQuart,
    EaseInQuint, EaseOutQuint, EaseInOutQuint,
    EaseInExpo, EaseOutExpo, EaseInOutExpo,
    EaseInCirc, EaseOutCirc, EaseInOutCirc,
    EaseInBack, EaseOutBack, EaseInOutBack,
    EaseInElastic, EaseOutElastic, EaseInOutElastic,
    EaseInBounce, EaseOutBounce, EaseInOutBounce,
    CubicBezier(f32, f32, f32, f32),
}

impl EasingKind {
    pub fn to_easing(&self) -> yumeri_animation::easing::Easing {
        use yumeri_animation::easing::Easing;
        match self {
            Self::Linear => Easing::Linear,
            Self::EaseInSine => Easing::EaseInSine,
            Self::EaseOutSine => Easing::EaseOutSine,
            Self::EaseInOutSine => Easing::EaseInOutSine,
            Self::EaseInQuad => Easing::EaseInQuad,
            Self::EaseOutQuad => Easing::EaseOutQuad,
            Self::EaseInOutQuad => Easing::EaseInOutQuad,
            Self::EaseInCubic => Easing::EaseInCubic,
            Self::EaseOutCubic => Easing::EaseOutCubic,
            Self::EaseInOutCubic => Easing::EaseInOutCubic,
            Self::EaseInQuart => Easing::EaseInQuart,
            Self::EaseOutQuart => Easing::EaseOutQuart,
            Self::EaseInOutQuart => Easing::EaseInOutQuart,
            Self::EaseInQuint => Easing::EaseInQuint,
            Self::EaseOutQuint => Easing::EaseOutQuint,
            Self::EaseInOutQuint => Easing::EaseInOutQuint,
            Self::EaseInExpo => Easing::EaseInExpo,
            Self::EaseOutExpo => Easing::EaseOutExpo,
            Self::EaseInOutExpo => Easing::EaseInOutExpo,
            Self::EaseInCirc => Easing::EaseInCirc,
            Self::EaseOutCirc => Easing::EaseOutCirc,
            Self::EaseInOutCirc => Easing::EaseInOutCirc,
            Self::EaseInBack => Easing::EaseInBack,
            Self::EaseOutBack => Easing::EaseOutBack,
            Self::EaseInOutBack => Easing::EaseInOutBack,
            Self::EaseInElastic => Easing::EaseInElastic,
            Self::EaseOutElastic => Easing::EaseOutElastic,
            Self::EaseInOutElastic => Easing::EaseInOutElastic,
            Self::EaseInBounce => Easing::EaseInBounce,
            Self::EaseOutBounce => Easing::EaseOutBounce,
            Self::EaseInOutBounce => Easing::EaseInOutBounce,
            Self::CubicBezier(a, b, c, d) => Easing::CubicBezier(*a, *b, *c, *d),
        }
    }
}
