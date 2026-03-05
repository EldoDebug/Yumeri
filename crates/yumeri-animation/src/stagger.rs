use std::time::Duration;

use crate::easing::Easing;
use crate::interpolate::Interpolate;
use crate::tween::Tween;

/// Which end of the list to start staggering from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaggerFrom {
    First,
    Last,
    Center,
    Index(usize),
}

/// Configuration for stagger delay distribution.
pub struct StaggerConfig {
    /// Delay between each successive element.
    pub interval: Duration,
    /// Origin of the stagger wave.
    pub from: StaggerFrom,
    /// Optional easing applied to the delay distribution.
    pub easing: Option<Easing>,
}

/// Generate `count` tweens with staggered delays.
///
/// `factory(index)` produces a base `Tween<T>` for each element.
/// The stagger logic adds an incremental delay to each tween based on `config`.
pub fn stagger<T, F>(count: usize, config: StaggerConfig, factory: F) -> Vec<Tween<T>>
where
    T: Interpolate,
    F: Fn(usize) -> Tween<T>,
{
    if count == 0 {
        return Vec::new();
    }

    (0..count)
        .map(|i| {
            let mut tween = factory(i);
            let stagger_delay = compute_delay(i, count, &config);
            tween.delay += stagger_delay;
            tween
        })
        .collect()
}

fn compute_delay(index: usize, count: usize, config: &StaggerConfig) -> Duration {
    let distance = match config.from {
        StaggerFrom::First => index as f64,
        StaggerFrom::Last => (count - 1 - index) as f64,
        StaggerFrom::Center => {
            let center = (count - 1) as f64 / 2.0;
            (index as f64 - center).abs()
        }
        StaggerFrom::Index(origin) => (index as isize - origin as isize).unsigned_abs() as f64,
    };

    match &config.easing {
        None => {
            Duration::from_secs_f64(config.interval.as_secs_f64() * distance)
        }
        Some(easing) => {
            let max_distance = match config.from {
                StaggerFrom::First | StaggerFrom::Last => (count - 1) as f64,
                StaggerFrom::Center => (count - 1) as f64 / 2.0,
                StaggerFrom::Index(origin) => {
                    let to_start = origin as f64;
                    let to_end = (count - 1).abs_diff(origin) as f64;
                    to_start.max(to_end)
                }
            };
            if max_distance <= 0.0 {
                return Duration::ZERO;
            }
            let normalized = (distance / max_distance) as f32;
            let eased = easing.evaluate(normalized);
            // Total span = interval * max_distance, distribute by eased ratio
            Duration::from_secs_f64(
                config.interval.as_secs_f64() * max_distance * eased as f64,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tween::Tween;

    #[test]
    fn stagger_from_first() {
        let tweens = stagger(
            4,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::First,
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );

        assert_eq!(tweens.len(), 4);
        assert_eq!(tweens[0].delay, Duration::ZERO);
        assert_eq!(tweens[1].delay, Duration::from_millis(100));
        assert_eq!(tweens[2].delay, Duration::from_millis(200));
        assert_eq!(tweens[3].delay, Duration::from_millis(300));
    }

    #[test]
    fn stagger_from_last() {
        let tweens = stagger(
            3,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::Last,
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );

        assert_eq!(tweens[0].delay, Duration::from_millis(200));
        assert_eq!(tweens[1].delay, Duration::from_millis(100));
        assert_eq!(tweens[2].delay, Duration::ZERO);
    }

    #[test]
    fn stagger_from_center() {
        let tweens = stagger(
            5,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::Center,
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );

        // Center = index 2. Distances: [2, 1, 0, 1, 2]
        // delays = interval * distance: [200, 100, 0, 100, 200]
        assert_eq!(tweens[0].delay, Duration::from_millis(200));
        assert_eq!(tweens[1].delay, Duration::from_millis(100));
        assert_eq!(tweens[2].delay, Duration::ZERO);
        assert_eq!(tweens[3].delay, Duration::from_millis(100));
        assert_eq!(tweens[4].delay, Duration::from_millis(200));
    }

    #[test]
    fn stagger_empty() {
        let tweens = stagger(
            0,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::First,
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );
        assert!(tweens.is_empty());
    }

    #[test]
    fn stagger_single() {
        let tweens = stagger(
            1,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::First,
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );
        assert_eq!(tweens.len(), 1);
        assert_eq!(tweens[0].delay, Duration::ZERO);
    }

    #[test]
    fn stagger_from_index() {
        let tweens = stagger(
            5,
            StaggerConfig {
                interval: Duration::from_millis(100),
                from: StaggerFrom::Index(1),
                easing: None,
            },
            |_| Tween::new(0.0_f32, 1.0).build(),
        );

        // Origin = index 1. Distances: [1, 0, 1, 2, 3]
        assert_eq!(tweens[0].delay, Duration::from_millis(100));
        assert_eq!(tweens[1].delay, Duration::ZERO);
        assert_eq!(tweens[2].delay, Duration::from_millis(100));
        assert_eq!(tweens[3].delay, Duration::from_millis(200));
        assert_eq!(tweens[4].delay, Duration::from_millis(300));
    }
}
