use std::time::Duration;

use yumeri_animation::easing::Easing;
use yumeri_animation::handle::RawHandle;
use yumeri_types::Color;

#[derive(Clone)]
pub struct TransitionDef {
    pub property: TransitionProperty,
    pub duration: Duration,
    pub easing: Easing,
}

impl std::fmt::Debug for TransitionDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransitionDef")
            .field("property", &self.property)
            .field("duration", &self.duration)
            .field("easing", &"<Easing>")
            .finish()
    }
}

impl TransitionDef {
    pub fn new(property: TransitionProperty) -> Self {
        Self {
            property,
            duration: Duration::from_millis(200),
            easing: Easing::EaseInOutCubic,
        }
    }

    pub fn duration(mut self, d: Duration) -> Self {
        self.duration = d;
        self
    }

    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration = Duration::from_millis(ms);
        self
    }

    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionProperty {
    Opacity,
    BackgroundColor,
    Width,
    Height,
    CornerRadius,
}

#[derive(Clone, Debug)]
pub struct TransitionSnapshot {
    pub opacity: f32,
    pub background: Option<Color>,
    pub width: f32,
    pub height: f32,
    pub corner_radius: f32,
}

#[allow(dead_code)]
pub(crate) struct ActiveTransition {
    pub property: TransitionProperty,
    pub handle: RawHandle,
}
