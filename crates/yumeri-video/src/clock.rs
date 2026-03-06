use std::time::Instant;

use yumeri_audio::AudioHandle;

/// Presentation clock for A/V synchronization.
/// Uses the audio track position as master clock when available,
/// falling back to system time otherwise.
pub(crate) struct PresentationClock {
    audio_handle: Option<AudioHandle>,
    start_time: Instant,
    offset: f64,
}

impl PresentationClock {
    pub fn new(audio_handle: Option<AudioHandle>) -> Self {
        Self {
            audio_handle,
            start_time: Instant::now(),
            offset: 0.0,
        }
    }

    /// Current playback time in seconds.
    pub fn current_time(&self) -> f64 {
        if let Some(ref ah) = self.audio_handle {
            ah.position_secs()
        } else {
            self.start_time.elapsed().as_secs_f64() + self.offset
        }
    }

    /// Reset clock after a seek to `position_secs`.
    pub fn reset(&mut self, position_secs: f64) {
        self.start_time = Instant::now();
        self.offset = position_secs;
    }

    /// Pause the clock (for system-time mode).
    pub fn pause(&mut self) {
        if self.audio_handle.is_none() {
            self.offset += self.start_time.elapsed().as_secs_f64();
            self.start_time = Instant::now();
        }
    }

    /// Resume the clock (for system-time mode).
    pub fn resume(&mut self) {
        if self.audio_handle.is_none() {
            self.start_time = Instant::now();
        }
    }
}
