use std::collections::HashMap;
use std::time::Duration;

use crate::animator::Animator;
use crate::handle::AnimationId;
use crate::interpolate::Interpolate;
use crate::keyframes::Keyframes;
use crate::playback::{Direction, LoopMode};
use crate::tween::Tween;

/// Time offset for a timeline entry.
pub enum TimeOffset {
    /// Absolute time from the timeline start.
    At(Duration),
    /// Same as `At(Duration::ZERO)`.
    Start,
    /// After the previous animation ends.
    AfterPrevious,
    /// After the previous animation ends + delay.
    AfterPreviousWithDelay(Duration),
    /// Starts at the same time as the previous animation.
    WithPrevious,
    /// At a named label's position.
    AtLabel(String),
}

/// A timeline that orchestrates multiple animations on a shared time axis.
pub struct Timeline {
    pub(crate) entries: Vec<TimelineEntry>,
    pub(crate) loop_mode: LoopMode,
    pub(crate) direction: Direction,
}

pub(crate) struct TimelineEntry {
    pub(crate) anim_id: AnimationId,
    pub(crate) resolved_offset: Duration,
    pub(crate) duration: Duration,
}

impl Timeline {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> TimelineBuilder {
        TimelineBuilder {
            pending: Vec::new(),
            loop_mode: LoopMode::None,
            direction: Direction::Normal,
        }
    }

    /// Total duration of this timeline (max of offset + anim_duration across all entries).
    pub fn total_duration(&self) -> Duration {
        self.entries
            .iter()
            .map(|e| e.resolved_offset + e.duration)
            .max()
            .unwrap_or(Duration::ZERO)
    }
}

type AnimRegistrar = Box<dyn FnOnce(&mut Animator) -> (AnimationId, Duration)>;

enum PendingItem {
    Animation {
        registrar: AnimRegistrar,
        offset: TimeOffset,
    },
    Label {
        name: String,
    },
}

pub struct TimelineBuilder {
    pending: Vec<PendingItem>,
    loop_mode: LoopMode,
    direction: Direction,
}

impl TimelineBuilder {
    pub fn add<T: Interpolate>(mut self, tween: Tween<T>, offset: TimeOffset) -> Self {
        let dur = tween.delay() + tween.duration();
        self.pending.push(PendingItem::Animation {
            registrar: Box::new(move |animator: &mut Animator| {
                let handle = animator.play_timeline_child(tween);
                (handle.id, dur)
            }),
            offset,
        });
        self
    }

    pub fn add_keyframes<T: Interpolate>(
        mut self,
        kf: Keyframes<T>,
        offset: TimeOffset,
    ) -> Self {
        let dur = kf.delay() + kf.duration();
        self.pending.push(PendingItem::Animation {
            registrar: Box::new(move |animator: &mut Animator| {
                let handle = animator.play_timeline_child(kf);
                (handle.id, dur)
            }),
            offset,
        });
        self
    }

    /// Shorthand: add after previous animation ends.
    pub fn then<T: Interpolate>(self, tween: Tween<T>) -> Self {
        self.add(tween, TimeOffset::AfterPrevious)
    }

    /// Shorthand: add at the same time as previous animation.
    pub fn with<T: Interpolate>(self, tween: Tween<T>) -> Self {
        self.add(tween, TimeOffset::WithPrevious)
    }

    /// Mark the current timeline position with a named label.
    /// Later entries can reference this position with `TimeOffset::AtLabel`.
    pub fn label(mut self, name: &str) -> Self {
        self.pending.push(PendingItem::Label {
            name: name.to_string(),
        });
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

    /// Build the timeline, registering all child animations in the given `Animator`.
    pub fn build(self, animator: &mut Animator) -> Timeline {
        let mut entries = Vec::new();
        let mut labels: HashMap<String, Duration> = HashMap::new();
        let mut prev_offset = Duration::ZERO;
        let mut prev_duration = Duration::ZERO;

        for item in self.pending {
            match item {
                PendingItem::Label { name } => {
                    // Record current timeline position as a label
                    let position = prev_offset + prev_duration;
                    labels.insert(name, position);
                }
                PendingItem::Animation { registrar, offset } => {
                    let resolved_offset = match &offset {
                        TimeOffset::At(d) => *d,
                        TimeOffset::Start => Duration::ZERO,
                        TimeOffset::AfterPrevious => prev_offset + prev_duration,
                        TimeOffset::AfterPreviousWithDelay(delay) => {
                            prev_offset + prev_duration + *delay
                        }
                        TimeOffset::WithPrevious => prev_offset,
                        TimeOffset::AtLabel(name) => *labels
                            .get(name)
                            .unwrap_or_else(|| panic!("timeline label '{name}' not found")),
                    };

                    let (anim_id, anim_dur) = registrar(animator);

                    prev_offset = resolved_offset;
                    prev_duration = anim_dur;

                    entries.push(TimelineEntry {
                        anim_id,
                        resolved_offset,
                        duration: anim_dur,
                    });
                }
            }
        }

        Timeline {
            entries,
            loop_mode: self.loop_mode,
            direction: self.direction,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_sequential() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::Start,
            )
            .then(Tween::new(0.0_f32, 200.0).duration_ms(500).build())
            .build(&mut animator);

        assert_eq!(tl.entries.len(), 2);
        assert_eq!(tl.entries[0].resolved_offset, Duration::ZERO);
        assert_eq!(tl.entries[1].resolved_offset, Duration::from_millis(500));
        assert_eq!(tl.total_duration(), Duration::from_millis(1000));
    }

    #[test]
    fn timeline_parallel() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::Start,
            )
            .with(Tween::new(0.0_f32, 200.0).duration_ms(300).build())
            .build(&mut animator);

        assert_eq!(tl.entries.len(), 2);
        assert_eq!(tl.entries[0].resolved_offset, Duration::ZERO);
        assert_eq!(tl.entries[1].resolved_offset, Duration::ZERO);
        assert_eq!(tl.total_duration(), Duration::from_millis(500));
    }

    #[test]
    fn timeline_after_with_delay() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::Start,
            )
            .add(
                Tween::new(0.0_f32, 200.0).duration_ms(300).build(),
                TimeOffset::AfterPreviousWithDelay(Duration::from_millis(100)),
            )
            .build(&mut animator);

        assert_eq!(tl.entries[1].resolved_offset, Duration::from_millis(600));
        assert_eq!(tl.total_duration(), Duration::from_millis(900));
    }

    #[test]
    fn timeline_at_absolute() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::At(Duration::from_millis(200)),
            )
            .build(&mut animator);

        assert_eq!(tl.entries[0].resolved_offset, Duration::from_millis(200));
        assert_eq!(tl.total_duration(), Duration::from_millis(700));
    }

    #[test]
    fn timeline_label() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::Start,
            )
            .label("intro_end")
            .then(Tween::new(0.0_f32, 200.0).duration_ms(300).build())
            .add(
                Tween::new(0.0_f32, 50.0).duration_ms(200).build(),
                TimeOffset::AtLabel("intro_end".to_string()),
            )
            .build(&mut animator);

        assert_eq!(tl.entries.len(), 3);
        // First anim at 0
        assert_eq!(tl.entries[0].resolved_offset, Duration::ZERO);
        // Second anim after first (AfterPrevious)
        assert_eq!(tl.entries[1].resolved_offset, Duration::from_millis(500));
        // Third anim at the label (which is at 500ms, after the first anim)
        assert_eq!(tl.entries[2].resolved_offset, Duration::from_millis(500));
    }

    #[test]
    fn timeline_playback() {
        let mut animator = Animator::new();

        let tl = Timeline::new()
            .add(
                Tween::new(0.0_f32, 100.0).duration_ms(500).build(),
                TimeOffset::Start,
            )
            .then(Tween::new(100.0_f32, 200.0).duration_ms(500).build())
            .build(&mut animator);

        let _tl_handle = animator.play_timeline(tl);

        // At 250ms, first anim should be at 50%
        animator.update(Duration::from_millis(250));
        // At 750ms, second anim should be at 50% (local time = 250ms)
        animator.update(Duration::from_millis(500));
    }
}
