#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EyeState {
    First,
    Interval,
    Closing,
    Closed,
    Opening,
}

#[derive(Debug, Clone)]
pub struct EyeBlink {
    state: EyeState,
    next_blinking_time: f32,
    state_start_time_seconds: f32,
    blinking_interval_seconds: f32,
    closing_seconds: f32,
    closed_seconds: f32,
    opening_seconds: f32,
    user_time_seconds: f32,
}

impl Default for EyeBlink {
    /// Create an eye blink state machine using Cubism SDK default timings.
    fn default() -> Self {
        Self {
            state: EyeState::First,
            next_blinking_time: 0.0,
            state_start_time_seconds: 0.0,
            blinking_interval_seconds: 4.0,
            closing_seconds: 0.1,
            closed_seconds: 0.05,
            opening_seconds: 0.15,
            user_time_seconds: 0.0,
        }
    }
}

impl EyeBlink {
    /// Set the average interval between blinks (seconds).
    pub fn set_blinking_interval(&mut self, blinking_interval_seconds: f32) {
        self.blinking_interval_seconds = blinking_interval_seconds.max(0.0);
    }

    /// Set the closing/closed/opening durations (seconds).
    pub fn set_blinking_settings(
        &mut self,
        closing_seconds: f32,
        closed_seconds: f32,
        opening_seconds: f32,
    ) {
        self.closing_seconds = closing_seconds.max(0.0);
        self.closed_seconds = closed_seconds.max(0.0);
        self.opening_seconds = opening_seconds.max(0.0);
    }

    /// Advance the blink state machine and return the eye-open parameter value.
    pub fn update(&mut self, delta_time_seconds: f32) -> f32 {
        const CLOSE_IF_ZERO: bool = true;

        self.user_time_seconds += delta_time_seconds;

        let mut parameter_value;
        match self.state {
            EyeState::Closing => {
                let mut t = if self.closing_seconds > 0.0 {
                    (self.user_time_seconds - self.state_start_time_seconds) / self.closing_seconds
                } else {
                    1.0
                };
                if t >= 1.0 {
                    t = 1.0;
                    self.state = EyeState::Closed;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                parameter_value = 1.0 - t;
            }
            EyeState::Closed => {
                let t = if self.closed_seconds > 0.0 {
                    (self.user_time_seconds - self.state_start_time_seconds) / self.closed_seconds
                } else {
                    1.0
                };
                if t >= 1.0 {
                    self.state = EyeState::Opening;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                parameter_value = 0.0;
            }
            EyeState::Opening => {
                let mut t = if self.opening_seconds > 0.0 {
                    (self.user_time_seconds - self.state_start_time_seconds) / self.opening_seconds
                } else {
                    1.0
                };
                if t >= 1.0 {
                    t = 1.0;
                    self.state = EyeState::Interval;
                    self.next_blinking_time = self.determine_next_blinking_timing();
                }
                parameter_value = t;
            }
            EyeState::Interval => {
                if self.next_blinking_time < self.user_time_seconds {
                    self.state = EyeState::Closing;
                    self.state_start_time_seconds = self.user_time_seconds;
                }
                parameter_value = 1.0;
            }
            EyeState::First => {
                self.state = EyeState::Interval;
                self.next_blinking_time = self.determine_next_blinking_timing();
                parameter_value = 1.0;
            }
        }

        if !CLOSE_IF_ZERO {
            parameter_value = -parameter_value;
        }

        parameter_value
    }

    /// Compute the next blink start time using a uniform random offset.
    fn determine_next_blinking_timing(&self) -> f32 {
        let r: f32 = rand::random::<f32>().clamp(0.0, 1.0);
        self.user_time_seconds + (r * (2.0 * self.blinking_interval_seconds - 1.0))
    }
}
