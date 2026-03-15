use std::time::Duration;

use yumeri_animation::easing::Easing;

/// Defines a CSS-like transition for a style property.
/// Currently parsed by the template system but not yet wired to the
/// animation runtime. Stored on [`crate::style::Style::transitions`].
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
    Translate,
    Scale,
    Rotation,
}
