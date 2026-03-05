use std::time::Duration;

use crate::easing::Easing;
use crate::interpolate::Interpolate;
use crate::playback::{Direction, LoopMode};

/// Multi-stop keyframe animation.
pub struct Keyframes<T: Interpolate> {
    pub(crate) stops: Vec<KeyframeStop<T>>,
    pub(crate) duration: Duration,
    pub(crate) delay: Duration,
    pub(crate) loop_mode: LoopMode,
    pub(crate) direction: Direction,
}

pub struct KeyframeStop<T> {
    pub progress: f32,
    pub value: T,
    /// Easing from this stop to the next.
    pub easing: Easing,
}

impl<T: Interpolate> Keyframes<T> {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> KeyframesBuilder<T> {
        KeyframesBuilder {
            stops: Vec::new(),
            duration: Duration::from_millis(300),
            delay: Duration::ZERO,
            loop_mode: LoopMode::None,
            direction: Direction::Normal,
        }
    }

    /// Sample the keyframes at normalized progress `t` in `[0, 1]`.
    pub fn sample(&self, t: f32) -> T {
        let t = t.clamp(0.0, 1.0);

        if self.stops.is_empty() {
            panic!("Keyframes has no stops");
        }

        if self.stops.len() == 1 {
            return self.stops[0].value.clone();
        }

        // Find the segment: the pair of stops that bracket `t`
        if t <= self.stops[0].progress {
            return self.stops[0].value.clone();
        }
        if t >= self.stops.last().unwrap().progress {
            return self.stops.last().unwrap().value.clone();
        }

        for i in 0..self.stops.len() - 1 {
            let a = &self.stops[i];
            let b = &self.stops[i + 1];
            if t >= a.progress && t <= b.progress {
                let segment_len = b.progress - a.progress;
                let local_t = if segment_len > 0.0 {
                    (t - a.progress) / segment_len
                } else {
                    1.0
                };
                let eased = a.easing.evaluate(local_t);
                return a.value.lerp(&b.value, eased);
            }
        }

        self.stops.last().unwrap().value.clone()
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

pub struct KeyframesBuilder<T: Interpolate> {
    stops: Vec<KeyframeStop<T>>,
    duration: Duration,
    delay: Duration,
    loop_mode: LoopMode,
    direction: Direction,
}

impl<T: Interpolate> KeyframesBuilder<T> {
    pub fn stop(mut self, progress: f32, value: T) -> Self {
        self.stops.push(KeyframeStop {
            progress,
            value,
            easing: Easing::Linear,
        });
        self
    }

    pub fn stop_with_easing(mut self, progress: f32, value: T, easing: Easing) -> Self {
        self.stops.push(KeyframeStop {
            progress,
            value,
            easing,
        });
        self
    }

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

    pub fn loop_mode(mut self, m: LoopMode) -> Self {
        self.loop_mode = m;
        self
    }

    pub fn direction(mut self, d: Direction) -> Self {
        self.direction = d;
        self
    }

    pub fn build(mut self) -> Keyframes<T> {
        assert!(
            !self.stops.is_empty(),
            "Keyframes must have at least one stop"
        );
        self.stops
            .sort_by(|a, b| a.progress.total_cmp(&b.progress));
        Keyframes {
            stops: self.stops,
            duration: self.duration,
            delay: self.delay,
            loop_mode: self.loop_mode,
            direction: self.direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframes_linear_three_stops() {
        let kf = Keyframes::new()
            .stop(0.0, 0.0_f32)
            .stop(0.5, 50.0)
            .stop(1.0, 100.0)
            .build();

        assert_eq!(kf.sample(0.0), 0.0);
        assert_eq!(kf.sample(0.25), 25.0);
        assert_eq!(kf.sample(0.5), 50.0);
        assert_eq!(kf.sample(0.75), 75.0);
        assert_eq!(kf.sample(1.0), 100.0);
    }

    #[test]
    fn keyframes_with_easing() {
        let kf = Keyframes::new()
            .stop_with_easing(0.0, 0.0_f32, Easing::EaseInQuad)
            .stop(1.0, 100.0)
            .build();

        // EaseInQuad at t=0.5 → 0.25 → value = 25.0
        let v = kf.sample(0.5);
        assert!((v - 25.0).abs() < 0.01);
    }

    #[test]
    fn keyframes_unordered_stops() {
        // Builder should sort stops by progress
        let kf = Keyframes::new()
            .stop(1.0, 100.0_f32)
            .stop(0.0, 0.0)
            .stop(0.5, 50.0)
            .build();

        assert_eq!(kf.sample(0.25), 25.0);
    }

    #[test]
    fn keyframes_clamp_t() {
        let kf = Keyframes::new()
            .stop(0.0, 0.0_f32)
            .stop(1.0, 100.0)
            .build();

        assert_eq!(kf.sample(-0.5), 0.0);
        assert_eq!(kf.sample(1.5), 100.0);
    }
}
