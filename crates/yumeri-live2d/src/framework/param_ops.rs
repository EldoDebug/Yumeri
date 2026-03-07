use crate::core::Parameters;

/// Clamp a value into the inclusive `[0, 1]` range.
pub fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// Clamp `value` into the parameter's valid range, repeating when the parameter is marked as repeat.
fn clamp_or_repeat_parameter(params: &Parameters<'_>, index: usize, value: f32) -> f32 {
    let min = params.minimum_values()[index];
    let max = params.maximum_values()[index];
    if params.is_repeat(index) {
        repeat_value(min, max, value)
    } else {
        value.clamp(min, max)
    }
}

/// Wrap `value` into the `[min, max]` range (inclusive) using modular arithmetic.
fn repeat_value(min: f32, max: f32, value: f32) -> f32 {
    let size = max - min;
    if !(size > 0.0) {
        return value.clamp(min.min(max), min.max(max));
    }

    let mut v = value;
    if v > max {
        let over = (v - max).rem_euclid(size);
        v = if over.is_finite() { min + over } else { max };
    }
    if v < min {
        let over = (min - v).rem_euclid(size);
        v = if over.is_finite() { max - over } else { min };
    }
    v
}

/// Sine-based easing on the inclusive `[0, 1]` domain.
pub fn easing_sine01(t: f32) -> f32 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= 1.0 {
        return 1.0;
    }
    0.5 - 0.5 * (t * core::f32::consts::PI).cos()
}

/// Blend the parameter at `index` toward `value` with `weight` in `[0, 1]`.
pub fn set_parameter_value(params: &mut Parameters<'_>, index: usize, value: f32, weight: f32) {
    let weight = weight.clamp(0.0, 1.0);
    let value = clamp_or_repeat_parameter(params, index, value);
    let cur = params.values()[index];
    let new_value = cur * (1.0 - weight) + value * weight;
    params.values_mut()[index] = clamp_or_repeat_parameter(params, index, new_value);
}

/// Add `delta` to the parameter at `index` with `weight` in `[0, 1]`.
pub fn add_parameter_value(params: &mut Parameters<'_>, index: usize, delta: f32, weight: f32) {
    let weight = weight.clamp(0.0, 1.0);
    let cur = params.values()[index];
    let new_value = cur + delta * weight;
    params.values_mut()[index] = clamp_or_repeat_parameter(params, index, new_value);
}

/// Multiply the parameter at `index` by `multiplier` with `weight` in `[0, 1]`.
pub fn multiply_parameter_value(
    params: &mut Parameters<'_>,
    index: usize,
    multiplier: f32,
    weight: f32,
) {
    let weight = weight.clamp(0.0, 1.0);
    let cur = params.values()[index];
    let m = 1.0 + (multiplier - 1.0) * weight;
    let new_value = cur * m;
    params.values_mut()[index] = clamp_or_repeat_parameter(params, index, new_value);
}

/// Write a clamped opacity value for the part at `index`.
pub fn set_part_opacity(opacities: &mut [f32], index: usize, value: f32) {
    opacities[index] = clamp01(value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeat_value_wraps() {
        assert!((repeat_value(0.0, 10.0, 11.0) - 1.0).abs() < 1e-6);
        assert!((repeat_value(0.0, 10.0, -1.0) - 9.0).abs() < 1e-6);
        assert!((repeat_value(-1.0, 1.0, 1.5) - -0.5).abs() < 1e-6);
    }
}
