#[derive(Debug, Clone, Copy)]
pub struct TargetPoint {
    target_x: f32,
    target_y: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    last_time_seconds: f32,
    user_time_seconds: f32,
}

impl Default for TargetPoint {
    /// Create a target point controller at the origin.
    fn default() -> Self {
        Self {
            target_x: 0.0,
            target_y: 0.0,
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            last_time_seconds: 0.0,
            user_time_seconds: 0.0,
        }
    }
}

impl TargetPoint {
    /// Set the desired target position.
    pub fn set(&mut self, x: f32, y: f32) {
        self.target_x = x;
        self.target_y = y;
    }

    /// Advance the controller toward the target using a critically-damped style update.
    pub fn update(&mut self, delta_time_seconds: f32) {
        const FRAME_RATE: f32 = 30.0;
        const EPSILON: f32 = 0.01;

        self.user_time_seconds += delta_time_seconds;

        const FACE_PARAM_MAX_V: f32 = 40.0 / 10.0;
        let max_v = FACE_PARAM_MAX_V * 1.0 / FRAME_RATE;

        if self.last_time_seconds == 0.0 {
            self.last_time_seconds = self.user_time_seconds;
            return;
        }

        let delta_time_weight = (self.user_time_seconds - self.last_time_seconds) * FRAME_RATE;
        self.last_time_seconds = self.user_time_seconds;

        let time_to_max_speed = 0.15;
        let frame_to_max_speed = time_to_max_speed * FRAME_RATE;
        let max_a = delta_time_weight * max_v / frame_to_max_speed;

        let dx = self.target_x - self.x;
        let dy = self.target_y - self.y;
        if dx.abs() <= EPSILON && dy.abs() <= EPSILON {
            return;
        }

        let d = (dx * dx + dy * dy).sqrt();
        let vx = max_v * dx / d;
        let vy = max_v * dy / d;

        let mut ax = vx - self.vx;
        let mut ay = vy - self.vy;
        let a = (ax * ax + ay * ay).sqrt();
        if a < -max_a || a > max_a {
            ax *= max_a / a;
            ay *= max_a / a;
        }

        self.vx += ax;
        self.vy += ay;

        let max_v2 = 0.5 * (((max_a * max_a) + 16.0 * max_a * d - 8.0 * max_a * d).sqrt() - max_a);
        let cur_v = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if cur_v > max_v2 {
            self.vx *= max_v2 / cur_v;
            self.vy *= max_v2 / cur_v;
        }

        self.x += self.vx;
        self.y += self.vy;
    }

    /// Return the current X position.
    pub fn x(&self) -> f32 {
        self.x
    }

    /// Return the current Y position.
    pub fn y(&self) -> f32 {
        self.y
    }
}
