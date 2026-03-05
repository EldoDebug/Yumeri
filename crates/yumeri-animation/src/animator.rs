use std::any::Any;
use std::collections::HashMap;
use std::time::Duration;

use crate::handle::{AnimationId, Handle, RawHandle, TimelineHandle, TimelineId};
use crate::interpolate::Interpolate;
use crate::keyframes::Keyframes;
use crate::playback::{Direction, LoopMode, PlaybackState};
use crate::timeline::Timeline;
use crate::tween::Tween;

/// Events emitted by the animator each frame.
#[derive(Debug, Clone)]
pub enum AnimationEvent {
    Started(RawHandle),
    Completed(RawHandle),
    Looped { handle: RawHandle, count: u32 },
    Cancelled(RawHandle),
}

/// Central runtime that manages all active animations.
pub struct Animator {
    animations: HashMap<AnimationId, AnimationEntry>,
    timelines: HashMap<TimelineId, TimelineState>,
    events: Vec<AnimationEvent>,
    // Reusable scratch buffer for per-frame completed handles
    completed_buf: Vec<RawHandle>,
    next_id: u64,
}

struct AnimationEntry {
    anim: Box<dyn AnyAnimation>,
    timeline_owned: bool,
}

impl Animator {
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            timelines: HashMap::new(),
            events: Vec::new(),
            completed_buf: Vec::new(),
            next_id: 0,
        }
    }

    /// Register and start playing a `Tween`.
    pub fn play<T: Interpolate>(&mut self, tween: Tween<T>) -> Handle<T> {
        let handle = self.register(tween, false);
        self.events.push(AnimationEvent::Started(handle.raw()));
        handle
    }

    /// Register and start playing `Keyframes`.
    pub fn play_keyframes<T: Interpolate>(&mut self, kf: Keyframes<T>) -> Handle<T> {
        let handle = self.register(kf, false);
        self.events.push(AnimationEvent::Started(handle.raw()));
        handle
    }

    /// Register a source owned by a timeline (not advanced independently).
    pub(crate) fn play_timeline_child<T: Interpolate>(
        &mut self,
        source: impl AnimationSource<T> + Send + Sync + 'static,
    ) -> Handle<T> {
        self.register(source, true)
    }

    fn register<T: Interpolate>(
        &mut self,
        source: impl AnimationSource<T> + Send + Sync + 'static,
        timeline_owned: bool,
    ) -> Handle<T> {
        let id = self.alloc_id();
        let handle = Handle::new(id);
        let anim = AnimationState::new(source);
        self.animations.insert(
            id,
            AnimationEntry {
                anim: Box::new(anim),
                timeline_owned,
            },
        );
        handle
    }

    /// Register and start playing a `Timeline`.
    pub fn play_timeline(&mut self, timeline: Timeline) -> TimelineHandle {
        let tl_id = TimelineId(self.next_id);
        self.next_id += 1;

        let entries: Vec<_> = timeline
            .entries
            .iter()
            .map(|e| TimelineEntryState {
                anim_id: e.anim_id,
                offset_secs: e.resolved_offset.as_secs_f64(),
                duration_secs: e.duration.as_secs_f64(),
            })
            .collect();

        let state = TimelineState {
            entries,
            elapsed_secs: 0.0,
            total_duration_secs: timeline.total_duration().as_secs_f64(),
            loop_mode: timeline.loop_mode,
            direction: timeline.direction,
            playback: PlaybackState::Playing,
            loop_count: 0,
            speed: 1.0,
        };

        self.timelines.insert(tl_id, state);
        TimelineHandle { id: tl_id }
    }

    /// Advance all animations by `dt`. Call once per frame.
    pub fn update(&mut self, dt: Duration) {
        let dt_secs = dt.as_secs_f64();

        // Update standalone animations (skip timeline-owned ones)
        self.completed_buf.clear();
        for (&id, entry) in &mut self.animations {
            if entry.timeline_owned {
                continue;
            }
            if entry.anim.playback_state() != PlaybackState::Playing {
                continue;
            }
            let raw = RawHandle { id };
            match entry.anim.advance(dt_secs) {
                Some(InternalEvent::Completed) => self.completed_buf.push(raw),
                Some(InternalEvent::Looped(count)) => {
                    self.events.push(AnimationEvent::Looped {
                        handle: raw,
                        count,
                    });
                }
                None => {}
            }
        }
        for &raw in &self.completed_buf {
            self.events.push(AnimationEvent::Completed(raw));
        }

        // Update timelines
        self.completed_buf.clear();
        for (&tl_id, state) in &mut self.timelines {
            if state.playback != PlaybackState::Playing {
                continue;
            }

            state.elapsed_secs += dt_secs * state.speed as f64;

            let total = state.total_duration_secs;
            if total <= 0.0 {
                state.playback = PlaybackState::Completed;
                self.completed_buf.push(RawHandle {
                    id: AnimationId(tl_id.0),
                });
                continue;
            }

            if state.elapsed_secs >= total {
                match state.loop_mode {
                    LoopMode::None => {
                        state.elapsed_secs = total;
                        state.playback = PlaybackState::Completed;
                    }
                    LoopMode::Count(n) => {
                        state.loop_count += 1;
                        if state.loop_count >= n {
                            state.elapsed_secs = total;
                            state.playback = PlaybackState::Completed;
                        } else {
                            state.elapsed_secs %= total;
                        }
                    }
                    LoopMode::Infinite => {
                        state.loop_count += 1;
                        state.elapsed_secs %= total;
                    }
                }
            }

            // Seek each child animation based on the timeline's current position
            let mut p = (state.elapsed_secs / total).clamp(0.0, 1.0);
            if is_reversed(state.direction, state.loop_count) {
                p = 1.0 - p;
            }
            let tl_time_secs = p * total;

            for entry in &state.entries {
                if let Some(anim_entry) = self.animations.get_mut(&entry.anim_id) {
                    if entry.duration_secs <= 0.0 {
                        continue;
                    }
                    let local_time = tl_time_secs - entry.offset_secs;
                    if local_time < 0.0 {
                        anim_entry.anim.seek(0.0);
                    } else if local_time >= entry.duration_secs {
                        anim_entry.anim.seek(1.0);
                    } else {
                        anim_entry.anim.seek((local_time / entry.duration_secs) as f32);
                    }
                }
            }
        }

        // Emit timeline completed events
        for &raw in &self.completed_buf {
            // Find matching timeline by ID
            let tl_id = TimelineId(raw.id.0);
            if let Some(state) = self.timelines.get(&tl_id) {
                for entry in &state.entries {
                    self.events
                        .push(AnimationEvent::Completed(RawHandle { id: entry.anim_id }));
                }
            }
        }
    }

    /// Get the current interpolated value.
    pub fn get<T: Interpolate>(&self, handle: Handle<T>) -> T {
        self.animations
            .get(&handle.id)
            .and_then(|e| e.anim.as_any().downcast_ref::<T>())
            .cloned()
            .expect("animation not found or type mismatch")
    }

    /// Check if the animation has completed.
    pub fn is_complete<T: Interpolate>(&self, handle: Handle<T>) -> bool {
        self.animations
            .get(&handle.id)
            .map(|e| e.anim.playback_state() == PlaybackState::Completed)
            .unwrap_or(true)
    }

    pub fn pause(&mut self, handle: RawHandle) {
        if let Some(e) = self.animations.get_mut(&handle.id) {
            e.anim.set_playback(PlaybackState::Paused);
        }
    }

    pub fn resume(&mut self, handle: RawHandle) {
        if let Some(e) = self.animations.get_mut(&handle.id)
            && e.anim.playback_state() == PlaybackState::Paused
        {
            e.anim.set_playback(PlaybackState::Playing);
        }
    }

    pub fn cancel(&mut self, handle: RawHandle) {
        if self.animations.remove(&handle.id).is_some() {
            self.events.push(AnimationEvent::Cancelled(handle));
        }
    }

    /// Seek to a normalized position `[0, 1]`.
    pub fn seek(&mut self, handle: RawHandle, progress: f32) {
        if let Some(e) = self.animations.get_mut(&handle.id) {
            e.anim.seek(progress.clamp(0.0, 1.0));
        }
    }

    /// Set playback speed multiplier.
    pub fn set_speed(&mut self, handle: RawHandle, speed: f32) {
        if let Some(e) = self.animations.get_mut(&handle.id) {
            e.anim.set_speed(speed);
        }
    }

    /// Drain all pending events.
    pub fn drain_events(&mut self) -> impl Iterator<Item = AnimationEvent> + '_ {
        self.events.drain(..)
    }

    /// Remove completed animations.
    pub fn gc(&mut self) {
        self.animations
            .retain(|_, e| e.anim.playback_state() != PlaybackState::Completed);
        self.timelines
            .retain(|_, state| state.playback != PlaybackState::Completed);
    }

    fn alloc_id(&mut self) -> AnimationId {
        let id = AnimationId(self.next_id);
        self.next_id += 1;
        id
    }
}

impl Default for Animator {
    fn default() -> Self {
        Self::new()
    }
}

// --- Shared direction helper ---

fn is_reversed(direction: Direction, loop_count: u32) -> bool {
    match direction {
        Direction::Normal => false,
        Direction::Reverse => true,
        Direction::Alternate => loop_count % 2 == 1,
        Direction::AlternateReverse => loop_count % 2 == 0,
    }
}

fn apply_direction(direction: Direction, loop_count: u32, progress: f32) -> f32 {
    if is_reversed(direction, loop_count) {
        1.0 - progress
    } else {
        progress
    }
}

// --- Animation source trait ---

/// Trait abstracting over `Tween<T>` and `Keyframes<T>`.
pub(crate) trait AnimationSource<T: Interpolate> {
    fn sample(&self, t: f32) -> T;
    fn delay_secs(&self) -> f64;
    fn duration_secs(&self) -> f64;
    fn total_secs(&self) -> f64 {
        self.delay_secs() + self.duration_secs()
    }
    fn loop_mode(&self) -> LoopMode;
    fn direction(&self) -> Direction;
}

impl<T: Interpolate> AnimationSource<T> for Tween<T> {
    fn sample(&self, t: f32) -> T {
        self.sample(t)
    }
    fn delay_secs(&self) -> f64 {
        self.delay().as_secs_f64()
    }
    fn duration_secs(&self) -> f64 {
        self.duration().as_secs_f64()
    }
    fn loop_mode(&self) -> LoopMode {
        self.loop_mode()
    }
    fn direction(&self) -> Direction {
        self.direction()
    }
}

impl<T: Interpolate> AnimationSource<T> for Keyframes<T> {
    fn sample(&self, t: f32) -> T {
        self.sample(t)
    }
    fn delay_secs(&self) -> f64 {
        self.delay().as_secs_f64()
    }
    fn duration_secs(&self) -> f64 {
        self.duration().as_secs_f64()
    }
    fn loop_mode(&self) -> LoopMode {
        self.loop_mode()
    }
    fn direction(&self) -> Direction {
        self.direction()
    }
}

// --- Internal animation trait (type-erased) ---

enum InternalEvent {
    Completed,
    Looped(u32),
}

trait AnyAnimation: Send + Sync {
    fn advance(&mut self, dt_secs: f64) -> Option<InternalEvent>;
    fn as_any(&self) -> &dyn Any;
    fn playback_state(&self) -> PlaybackState;
    fn set_playback(&mut self, state: PlaybackState);
    fn seek(&mut self, progress: f32);
    fn set_speed(&mut self, speed: f32);
}

// --- Unified animation state ---

struct AnimationState<S, T: Interpolate> {
    source: S,
    // Pre-cached durations (avoid repeated Duration→f64 conversion)
    delay_secs: f64,
    total_secs: f64,
    // Runtime state
    elapsed_secs: f64,
    current: T,
    playback: PlaybackState,
    loop_count: u32,
    speed: f32,
}

impl<S: AnimationSource<T>, T: Interpolate> AnimationState<S, T> {
    fn new(source: S) -> Self {
        let delay_secs = source.delay_secs();
        let total_secs = source.total_secs();
        let initial = source.sample(0.0);
        let mut s = Self {
            source,
            delay_secs,
            total_secs,
            elapsed_secs: 0.0,
            current: initial,
            playback: PlaybackState::Playing,
            loop_count: 0,
            speed: 1.0,
        };
        s.current = s.compute_current();
        s
    }

    fn compute_progress(&self) -> f32 {
        if self.elapsed_secs < self.delay_secs {
            return 0.0;
        }
        let dur_secs = self.total_secs - self.delay_secs;
        if dur_secs <= 0.0 {
            return 1.0;
        }
        (((self.elapsed_secs - self.delay_secs) / dur_secs) as f32).clamp(0.0, 1.0)
    }

    fn compute_current(&self) -> T {
        let progress = self.compute_progress();
        let directed = apply_direction(self.source.direction(), self.loop_count, progress);
        self.source.sample(directed)
    }
}

impl<S, T> AnyAnimation for AnimationState<S, T>
where
    S: AnimationSource<T> + Send + Sync + 'static,
    T: Interpolate,
{
    fn advance(&mut self, dt_secs: f64) -> Option<InternalEvent> {
        if self.playback != PlaybackState::Playing {
            return None;
        }

        self.elapsed_secs += dt_secs * self.speed as f64;

        let event = if self.elapsed_secs >= self.total_secs {
            match self.source.loop_mode() {
                LoopMode::None => {
                    self.elapsed_secs = self.total_secs;
                    self.playback = PlaybackState::Completed;
                    Some(InternalEvent::Completed)
                }
                LoopMode::Count(n) => {
                    self.loop_count += 1;
                    if self.loop_count >= n {
                        self.elapsed_secs = self.total_secs;
                        self.playback = PlaybackState::Completed;
                        Some(InternalEvent::Completed)
                    } else {
                        self.elapsed_secs %= self.total_secs;
                        Some(InternalEvent::Looped(self.loop_count))
                    }
                }
                LoopMode::Infinite => {
                    self.loop_count += 1;
                    self.elapsed_secs %= self.total_secs;
                    Some(InternalEvent::Looped(self.loop_count))
                }
            }
        } else {
            None
        };

        self.current = self.compute_current();
        event
    }

    fn as_any(&self) -> &dyn Any {
        &self.current as &dyn Any
    }

    fn playback_state(&self) -> PlaybackState {
        self.playback
    }

    fn set_playback(&mut self, state: PlaybackState) {
        self.playback = state;
    }

    fn seek(&mut self, progress: f32) {
        self.elapsed_secs = self.total_secs * progress as f64;
        self.current = self.compute_current();
    }

    fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }
}

// --- Timeline state ---

struct TimelineEntryState {
    anim_id: AnimationId,
    offset_secs: f64,
    duration_secs: f64,
}

struct TimelineState {
    entries: Vec<TimelineEntryState>,
    elapsed_secs: f64,
    total_duration_secs: f64,
    loop_mode: LoopMode,
    direction: Direction,
    playback: PlaybackState,
    loop_count: u32,
    speed: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animator_play_and_get() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .build(),
        );

        let v = animator.get(handle);
        assert!((v - 0.0).abs() < 0.01);

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 100.0).abs() < 0.01);
        assert!(animator.is_complete(handle));
    }

    #[test]
    fn animator_with_delay() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .delay_ms(500)
                .build(),
        );

        animator.update(Duration::from_millis(250));
        assert!((animator.get(handle) - 0.0).abs() < 0.01);

        animator.update(Duration::from_millis(750));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);
    }

    #[test]
    fn animator_loop_count() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .loop_mode(LoopMode::Count(2))
                .build(),
        );

        animator.update(Duration::from_millis(100));
        assert!(!animator.is_complete(handle));

        animator.update(Duration::from_millis(100));
        assert!(animator.is_complete(handle));
    }

    #[test]
    fn animator_pause_resume() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .build(),
        );

        animator.update(Duration::from_millis(500));
        let v_before_pause = animator.get(handle);

        animator.pause(handle.into());
        animator.update(Duration::from_millis(500));
        let v_after_pause = animator.get(handle);

        assert!((v_before_pause - v_after_pause).abs() < 0.01);

        animator.resume(handle.into());
        animator.update(Duration::from_millis(500));
        assert!(animator.is_complete(handle));
    }

    #[test]
    fn animator_seek() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .build(),
        );

        animator.seek(handle.into(), 0.5);
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);
    }

    #[test]
    fn animator_cancel() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .build(),
        );

        animator.cancel(handle.into());
        assert!(animator.is_complete(handle));
    }

    #[test]
    fn animator_events() {
        let mut animator = Animator::new();
        let _handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .build(),
        );

        let events: Vec<_> = animator.drain_events().collect();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AnimationEvent::Started(_)));

        animator.update(Duration::from_millis(100));
        let events: Vec<_> = animator.drain_events().collect();
        assert!(events.iter().any(|e| matches!(e, AnimationEvent::Completed(_))));
    }

    #[test]
    fn animator_gc() {
        let mut animator = Animator::new();
        let _ = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .build(),
        );

        animator.update(Duration::from_millis(100));
        animator.gc();
        assert!(animator.animations.is_empty());
    }

    #[test]
    fn animator_direction_reverse() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .direction(Direction::Reverse)
                .build(),
        );

        let v = animator.get(handle);
        assert!((v - 100.0).abs() < 0.01);

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 0.0).abs() < 0.01);
    }

    #[test]
    fn animator_keyframes() {
        let mut animator = Animator::new();
        let handle = animator.play_keyframes(
            Keyframes::new()
                .stop(0.0, 0.0_f32)
                .stop(0.5, 50.0)
                .stop(1.0, 100.0)
                .duration_ms(1000)
                .build(),
        );

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);

        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 100.0).abs() < 0.01);
    }

    #[test]
    fn animator_speed() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(1000)
                .build(),
        );

        animator.set_speed(handle.into(), 2.0);
        animator.update(Duration::from_millis(500));
        let v = animator.get(handle);
        assert!((v - 100.0).abs() < 0.01);
        assert!(animator.is_complete(handle));
    }

    #[test]
    fn animator_loop_infinite() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .loop_mode(LoopMode::Infinite)
                .build(),
        );

        for _ in 0..10 {
            animator.update(Duration::from_millis(100));
            assert!(!animator.is_complete(handle));
        }

        let events: Vec<_> = animator.drain_events().collect();
        assert!(events
            .iter()
            .any(|e| matches!(e, AnimationEvent::Looped { .. })));
    }

    #[test]
    fn animator_direction_alternate() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .loop_mode(LoopMode::Count(2))
                .direction(Direction::Alternate)
                .build(),
        );

        animator.update(Duration::from_millis(50));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);

        animator.update(Duration::from_millis(50));
        animator.update(Duration::from_millis(50));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);
    }

    #[test]
    fn animator_direction_alternate_reverse() {
        let mut animator = Animator::new();
        let handle = animator.play(
            Tween::new(0.0_f32, 100.0)
                .duration_ms(100)
                .loop_mode(LoopMode::Count(2))
                .direction(Direction::AlternateReverse)
                .build(),
        );

        let v = animator.get(handle);
        assert!((v - 100.0).abs() < 0.01);

        animator.update(Duration::from_millis(50));
        let v = animator.get(handle);
        assert!((v - 50.0).abs() < 1.0);
    }
}
