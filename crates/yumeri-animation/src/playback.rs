/// Controls how an animation loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    /// Play once and stop.
    None,
    /// Loop a fixed number of times.
    Count(u32),
    /// Loop forever.
    Infinite,
}

/// Controls the playback direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Normal,
    Reverse,
    /// Ping-pong: forward then backward each loop.
    Alternate,
    /// Reverse ping-pong: backward then forward each loop.
    AlternateReverse,
}

/// The current state of an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,
    Paused,
    Completed,
}

impl Default for LoopMode {
    fn default() -> Self {
        Self::None
    }
}

impl Default for Direction {
    fn default() -> Self {
        Self::Normal
    }
}
