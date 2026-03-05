use std::time::Duration;

use crate::easing::Easing;
use crate::interpolate::Interpolate;
use crate::playback::{Direction, LoopMode};

/// A single from→to interpolation with easing.
pub struct Tween<T: Interpolate> {
    pub(crate) from: T,
    pub(crate) to: T,
    pub(crate) duration: Duration,
    pub(crate) delay: Duration,
    pub(crate) easing: Easing,
    pub(crate) loop_mode: LoopMode,
    pub(crate) direction: Direction,
}

impl<T: Interpolate> Tween<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(from: T, to: T) -> TweenBuilder<T> {
        TweenBuilder {
            from,
            to,
            duration: Duration::from_millis(300),
            delay: Duration::ZERO,
            easing: Easing::Linear,
            loop_mode: LoopMode::None,
            direction: Direction::Normal,
        }
    }

    /// Sample the tween at a normalized progress `t` in `[0, 1]`.
    pub fn sample(&self, t: f32) -> T {
        let eased = self.easing.evaluate(t);
        self.from.lerp(&self.to, eased)
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn delay(&self) -> Duration {
        self.delay
    }

    pub fn loop_mode(&self) -> LoopMode {
        self.loop_mode
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }
}

pub struct TweenBuilder<T: Interpolate> {
    from: T,
    to: T,
    duration: Duration,
    delay: Duration,
    easing: Easing,
    loop_mode: LoopMode,
    direction: Direction,
}

impl<T: Interpolate> TweenBuilder<T> {
    pub fn duration(mut self, d: Duration) -> Self {
        self.duration = d;
        self
    }

    pub fn duration_ms(mut self, ms: u64) -> Self {
        self.duration = Duration::from_millis(ms);
        self
    }

    pub fn delay(mut self, d: Duration) -> Self {
        self.delay = d;
        self
    }

    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay = Duration::from_millis(ms);
        self
    }

    pub fn easing(mut self, e: Easing) -> Self {
        self.easing = e;
        self
    }

    pub fn loop_mode(mut self, m: LoopMode) -> Self {
        self.loop_mode = m;
        self
    }

    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }

    pub fn build(self) -> Tween<T> {
        Tween {
            from: self.from,
            to: self.to,
            duration: self.duration,
            delay: self.delay,
            easing: self.easing,
            loop_mode: self.loop_mode,
            direction: self.direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tween_sample_linear() {
        let tween = Tween::new(0.0_f32, 100.0).build();
        assert_eq!(tween.sample(0.0), 0.0);
        assert_eq!(tween.sample(0.5), 50.0);
        assert_eq!(tween.sample(1.0), 100.0);
    }

    #[test]
    fn tween_sample_with_easing() {
        let tween = Tween::new(0.0_f32, 100.0)
            .easing(Easing::EaseInQuad)
            .build();
        // EaseInQuad: t^2 → at 0.5, should be 25.0
        assert!((tween.sample(0.5) - 25.0).abs() < 0.01);
    }

    #[test]
    fn tween_builder_defaults() {
        let tween = Tween::new(0.0_f32, 1.0).build();
        assert_eq!(tween.duration(), Duration::from_millis(300));
        assert_eq!(tween.delay(), Duration::ZERO);
        assert_eq!(tween.loop_mode(), LoopMode::None);
        assert_eq!(tween.direction(), Direction::Normal);
    }

    #[test]
    fn tween_builder_custom() {
        let tween = Tween::new(0.0_f32, 1.0)
            .duration_ms(500)
            .delay_ms(100)
            .loop_mode(LoopMode::Count(3))
            .direction(Direction::Alternate)
            .build();
        assert_eq!(tween.duration(), Duration::from_millis(500));
        assert_eq!(tween.delay(), Duration::from_millis(100));
        assert_eq!(tween.loop_mode(), LoopMode::Count(3));
        assert_eq!(tween.direction(), Direction::Alternate);
    }
}
