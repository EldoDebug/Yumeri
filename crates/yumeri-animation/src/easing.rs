use std::f32::consts::PI;
use std::sync::Arc;

/// Easing functions compatible with easings.net + CSS cubic-bezier.
#[derive(Clone)]
pub enum Easing {
    Linear,

    EaseInSine,
    EaseOutSine,
    EaseInOutSine,

    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,

    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,

    EaseInQuart,
    EaseOutQuart,
    EaseInOutQuart,

    EaseInQuint,
    EaseOutQuint,
    EaseInOutQuint,

    EaseInExpo,
    EaseOutExpo,
    EaseInOutExpo,

    EaseInCirc,
    EaseOutCirc,
    EaseInOutCirc,

    EaseInBack,
    EaseOutBack,
    EaseInOutBack,

    EaseInElastic,
    EaseOutElastic,
    EaseInOutElastic,

    EaseInBounce,
    EaseOutBounce,
    EaseInOutBounce,

    /// CSS `cubic-bezier(x1, y1, x2, y2)`.
    CubicBezier(f32, f32, f32, f32),

    Custom(Arc<dyn Fn(f32) -> f32 + Send + Sync>),
}

impl Easing {
    /// Evaluate the easing function at `t` where `t` is in `[0, 1]`.
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,

            // Sine
            Self::EaseInSine => 1.0 - ((t * PI / 2.0).cos()),
            Self::EaseOutSine => (t * PI / 2.0).sin(),
            Self::EaseInOutSine => -(((PI * t).cos()) - 1.0) / 2.0,

            // Quad
            Self::EaseInQuad => t * t,
            Self::EaseOutQuad => 1.0 - (1.0 - t) * (1.0 - t),
            Self::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(2) / 2.0
                }
            }

            // Cubic
            Self::EaseInCubic => t * t * t,
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }

            // Quart
            Self::EaseInQuart => t * t * t * t,
            Self::EaseOutQuart => 1.0 - (1.0 - t).powi(4),
            Self::EaseInOutQuart => {
                if t < 0.5 {
                    8.0 * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(4) / 2.0
                }
            }

            // Quint
            Self::EaseInQuint => t * t * t * t * t,
            Self::EaseOutQuint => 1.0 - (1.0 - t).powi(5),
            Self::EaseInOutQuint => {
                if t < 0.5 {
                    16.0 * t * t * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(5) / 2.0
                }
            }

            // Expo
            Self::EaseInExpo => {
                if t == 0.0 {
                    0.0
                } else {
                    (2.0_f32).powf(10.0 * t - 10.0)
                }
            }
            Self::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - (2.0_f32).powf(-10.0 * t)
                }
            }
            Self::EaseInOutExpo => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else if t < 0.5 {
                    (2.0_f32).powf(20.0 * t - 10.0) / 2.0
                } else {
                    (2.0 - (2.0_f32).powf(-20.0 * t + 10.0)) / 2.0
                }
            }

            // Circ
            Self::EaseInCirc => 1.0 - (1.0 - t * t).sqrt(),
            Self::EaseOutCirc => (1.0 - (t - 1.0).powi(2)).sqrt(),
            Self::EaseInOutCirc => {
                if t < 0.5 {
                    (1.0 - (1.0 - (2.0 * t).powi(2)).sqrt()) / 2.0
                } else {
                    ((1.0 - (-2.0 * t + 2.0).powi(2)).sqrt() + 1.0) / 2.0
                }
            }

            // Back
            Self::EaseInBack => {
                let c1 = 1.70158_f32;
                let c3 = c1 + 1.0;
                c3 * t * t * t - c1 * t * t
            }
            Self::EaseOutBack => {
                let c1 = 1.70158_f32;
                let c3 = c1 + 1.0;
                1.0 + c3 * (t - 1.0).powi(3) + c1 * (t - 1.0).powi(2)
            }
            Self::EaseInOutBack => {
                let c1 = 1.70158_f32;
                let c2 = c1 * 1.525;
                if t < 0.5 {
                    ((2.0 * t).powi(2) * ((c2 + 1.0) * 2.0 * t - c2)) / 2.0
                } else {
                    ((2.0 * t - 2.0).powi(2) * ((c2 + 1.0) * (t * 2.0 - 2.0) + c2) + 2.0) / 2.0
                }
            }

            // Elastic
            Self::EaseInElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * PI) / 3.0;
                    -(2.0_f32).powf(10.0 * t - 10.0) * ((10.0 * t - 10.75) * c4).sin()
                }
            }
            Self::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c4 = (2.0 * PI) / 3.0;
                    (2.0_f32).powf(-10.0 * t) * ((10.0 * t - 0.75) * c4).sin() + 1.0
                }
            }
            Self::EaseInOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let c5 = (2.0 * PI) / 4.5;
                    if t < 0.5 {
                        -(2.0_f32).powf(20.0 * t - 10.0) * ((20.0 * t - 11.125) * c5).sin()
                            / 2.0
                    } else {
                        (2.0_f32).powf(-20.0 * t + 10.0) * ((20.0 * t - 11.125) * c5).sin()
                            / 2.0
                            + 1.0
                    }
                }
            }

            // Bounce
            Self::EaseInBounce => 1.0 - bounce_out(1.0 - t),
            Self::EaseOutBounce => bounce_out(t),
            Self::EaseInOutBounce => {
                if t < 0.5 {
                    (1.0 - bounce_out(1.0 - 2.0 * t)) / 2.0
                } else {
                    (1.0 + bounce_out(2.0 * t - 1.0)) / 2.0
                }
            }

            // CubicBezier - Newton-Raphson solver
            Self::CubicBezier(x1, y1, x2, y2) => cubic_bezier_evaluate(*x1, *y1, *x2, *y2, t),

            Self::Custom(f) => f(t),
        }
    }
}

fn bounce_out(t: f32) -> f32 {
    let n1 = 7.5625_f32;
    let d1 = 2.75_f32;

    if t < 1.0 / d1 {
        n1 * t * t
    } else if t < 2.0 / d1 {
        let t = t - 1.5 / d1;
        n1 * t * t + 0.75
    } else if t < 2.5 / d1 {
        let t = t - 2.25 / d1;
        n1 * t * t + 0.9375
    } else {
        let t = t - 2.625 / d1;
        n1 * t * t + 0.984375
    }
}

/// Evaluate CSS cubic-bezier(x1, y1, x2, y2) at position `x`.
/// Uses Newton-Raphson to find `t` from `x`, then computes `y(t)`.
fn cubic_bezier_evaluate(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Find t for given x using Newton-Raphson
    let mut t = x; // initial guess
    for _ in 0..8 {
        let x_at_t = sample_curve_x(x1, x2, t);
        let dx = sample_curve_dx(x1, x2, t);
        if dx.abs() < 1e-7 {
            break;
        }
        t -= (x_at_t - x) / dx;
        t = t.clamp(0.0, 1.0);
    }

    // Bisection fallback for robustness
    let x_at_t = sample_curve_x(x1, x2, t);
    if (x_at_t - x).abs() > 1e-5 {
        let mut lo = 0.0_f32;
        let mut hi = 1.0_f32;
        t = x;
        for _ in 0..20 {
            let x_at_t = sample_curve_x(x1, x2, t);
            if (x_at_t - x).abs() < 1e-7 {
                break;
            }
            if x_at_t < x {
                lo = t;
            } else {
                hi = t;
            }
            t = (lo + hi) / 2.0;
        }
    }

    sample_curve_y(y1, y2, t)
}

/// Parametric cubic bezier x(t) = 3*(1-t)^2*t*x1 + 3*(1-t)*t^2*x2 + t^3
fn sample_curve_x(x1: f32, x2: f32, t: f32) -> f32 {
    // Horner's form of: (1 - 3*x2 + 3*x1)*t^3 + (3*x2 - 6*x1)*t^2 + 3*x1*t
    (((1.0 - 3.0 * x2 + 3.0 * x1) * t + (3.0 * x2 - 6.0 * x1)) * t + 3.0 * x1) * t
}

/// Derivative dx/dt
fn sample_curve_dx(x1: f32, x2: f32, t: f32) -> f32 {
    (3.0 * (1.0 - 3.0 * x2 + 3.0 * x1)) * t * t + (2.0 * (3.0 * x2 - 6.0 * x1)) * t + 3.0 * x1
}

/// Parametric cubic bezier y(t)
fn sample_curve_y(y1: f32, y2: f32, t: f32) -> f32 {
    (((1.0 - 3.0 * y2 + 3.0 * y1) * t + (3.0 * y2 - 6.0 * y1)) * t + 3.0 * y1) * t
}

impl Default for Easing {
    fn default() -> Self {
        Self::Linear
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_standard_easings() -> Vec<(&'static str, Easing)> {
        vec![
            ("Linear", Easing::Linear),
            ("EaseInSine", Easing::EaseInSine),
            ("EaseOutSine", Easing::EaseOutSine),
            ("EaseInOutSine", Easing::EaseInOutSine),
            ("EaseInQuad", Easing::EaseInQuad),
            ("EaseOutQuad", Easing::EaseOutQuad),
            ("EaseInOutQuad", Easing::EaseInOutQuad),
            ("EaseInCubic", Easing::EaseInCubic),
            ("EaseOutCubic", Easing::EaseOutCubic),
            ("EaseInOutCubic", Easing::EaseInOutCubic),
            ("EaseInQuart", Easing::EaseInQuart),
            ("EaseOutQuart", Easing::EaseOutQuart),
            ("EaseInOutQuart", Easing::EaseInOutQuart),
            ("EaseInQuint", Easing::EaseInQuint),
            ("EaseOutQuint", Easing::EaseOutQuint),
            ("EaseInOutQuint", Easing::EaseInOutQuint),
            ("EaseInExpo", Easing::EaseInExpo),
            ("EaseOutExpo", Easing::EaseOutExpo),
            ("EaseInOutExpo", Easing::EaseInOutExpo),
            ("EaseInCirc", Easing::EaseInCirc),
            ("EaseOutCirc", Easing::EaseOutCirc),
            ("EaseInOutCirc", Easing::EaseInOutCirc),
            ("EaseInBack", Easing::EaseInBack),
            ("EaseOutBack", Easing::EaseOutBack),
            ("EaseInOutBack", Easing::EaseInOutBack),
            ("EaseInElastic", Easing::EaseInElastic),
            ("EaseOutElastic", Easing::EaseOutElastic),
            ("EaseInOutElastic", Easing::EaseInOutElastic),
            ("EaseInBounce", Easing::EaseInBounce),
            ("EaseOutBounce", Easing::EaseOutBounce),
            ("EaseInOutBounce", Easing::EaseInOutBounce),
        ]
    }

    #[test]
    fn all_easings_boundary_values() {
        for (name, easing) in all_standard_easings() {
            let at_0 = easing.evaluate(0.0);
            let at_1 = easing.evaluate(1.0);
            assert!(
                (at_0).abs() < 1e-4,
                "{name}: evaluate(0.0) = {at_0}, expected ≈ 0.0"
            );
            assert!(
                (at_1 - 1.0).abs() < 1e-4,
                "{name}: evaluate(1.0) = {at_1}, expected ≈ 1.0"
            );
        }
    }

    #[test]
    fn cubic_bezier_css_ease() {
        // CSS `ease` = cubic-bezier(0.25, 0.1, 0.25, 1.0)
        let ease = Easing::CubicBezier(0.25, 0.1, 0.25, 1.0);
        assert!((ease.evaluate(0.0)).abs() < 1e-4);
        assert!((ease.evaluate(1.0) - 1.0).abs() < 1e-4);
        // Midpoint should be > 0.5 for ease (accelerates then decelerates)
        let mid = ease.evaluate(0.5);
        assert!(mid > 0.5, "CSS ease at 0.5 should be > 0.5, got {mid}");
    }

    #[test]
    fn cubic_bezier_css_ease_in() {
        // CSS `ease-in` = cubic-bezier(0.42, 0, 1, 1)
        let ease_in = Easing::CubicBezier(0.42, 0.0, 1.0, 1.0);
        assert!((ease_in.evaluate(0.0)).abs() < 1e-4);
        assert!((ease_in.evaluate(1.0) - 1.0).abs() < 1e-4);
        let mid = ease_in.evaluate(0.5);
        assert!(
            mid < 0.5,
            "CSS ease-in at 0.5 should be < 0.5, got {mid}"
        );
    }

    #[test]
    fn cubic_bezier_css_ease_out() {
        // CSS `ease-out` = cubic-bezier(0, 0, 0.58, 1)
        let ease_out = Easing::CubicBezier(0.0, 0.0, 0.58, 1.0);
        assert!((ease_out.evaluate(0.0)).abs() < 1e-4);
        assert!((ease_out.evaluate(1.0) - 1.0).abs() < 1e-4);
        let mid = ease_out.evaluate(0.5);
        assert!(
            mid > 0.5,
            "CSS ease-out at 0.5 should be > 0.5, got {mid}"
        );
    }

    #[test]
    fn cubic_bezier_css_ease_in_out() {
        // CSS `ease-in-out` = cubic-bezier(0.42, 0, 0.58, 1)
        let ease_in_out = Easing::CubicBezier(0.42, 0.0, 0.58, 1.0);
        assert!((ease_in_out.evaluate(0.0)).abs() < 1e-4);
        assert!((ease_in_out.evaluate(1.0) - 1.0).abs() < 1e-4);
        let mid = ease_in_out.evaluate(0.5);
        assert!(
            (mid - 0.5).abs() < 0.05,
            "CSS ease-in-out at 0.5 should be ≈ 0.5, got {mid}"
        );
    }

    #[test]
    fn cubic_bezier_linear() {
        // cubic-bezier(0, 0, 1, 1) should be approximately linear
        let linear = Easing::CubicBezier(0.0, 0.0, 1.0, 1.0);
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let v = linear.evaluate(t);
            assert!(
                (v - t).abs() < 0.02,
                "cubic-bezier linear at {t} = {v}, expected ≈ {t}"
            );
        }
    }

    #[test]
    fn custom_easing() {
        let custom = Easing::Custom(Arc::new(|t| t * t));
        assert!((custom.evaluate(0.5) - 0.25).abs() < 1e-5);
    }

    #[test]
    fn easing_clamps_input() {
        let e = Easing::Linear;
        assert_eq!(e.evaluate(-0.5), 0.0);
        assert_eq!(e.evaluate(1.5), 1.0);
    }
}
