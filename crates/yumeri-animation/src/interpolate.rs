/// A type that can be linearly interpolated.
pub trait Interpolate: Clone + Send + Sync + 'static {
    fn lerp(&self, target: &Self, t: f32) -> Self;
}

impl Interpolate for f32 {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        self + (target - self) * t
    }
}

impl Interpolate for [f32; 2] {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        [
            self[0] + (target[0] - self[0]) * t,
            self[1] + (target[1] - self[1]) * t,
        ]
    }
}

impl Interpolate for [f32; 3] {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        [
            self[0] + (target[0] - self[0]) * t,
            self[1] + (target[1] - self[1]) * t,
            self[2] + (target[2] - self[2]) * t,
        ]
    }
}

impl Interpolate for [f32; 4] {
    fn lerp(&self, target: &Self, t: f32) -> Self {
        [
            self[0] + (target[0] - self[0]) * t,
            self[1] + (target[1] - self[1]) * t,
            self[2] + (target[2] - self[2]) * t,
            self[3] + (target[3] - self[3]) * t,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f32_lerp() {
        assert_eq!(0.0_f32.lerp(&100.0, 0.0), 0.0);
        assert_eq!(0.0_f32.lerp(&100.0, 0.5), 50.0);
        assert_eq!(0.0_f32.lerp(&100.0, 1.0), 100.0);
    }

    #[test]
    fn array2_lerp() {
        let a = [0.0, 10.0];
        let b = [100.0, 20.0];
        let result = a.lerp(&b, 0.5);
        assert_eq!(result, [50.0, 15.0]);
    }

    #[test]
    fn array3_lerp() {
        let a = [0.0, 0.0, 0.0];
        let b = [1.0, 2.0, 3.0];
        let result = a.lerp(&b, 0.5);
        assert_eq!(result, [0.5, 1.0, 1.5]);
    }

    #[test]
    fn array4_lerp() {
        let a = [0.0; 4];
        let b = [4.0, 8.0, 12.0, 16.0];
        let result = a.lerp(&b, 0.25);
        assert_eq!(result, [1.0, 2.0, 3.0, 4.0]);
    }
}
