use crate::framework::param_ops;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
pub enum MotionError {
    #[error("failed to parse motion3 json")]
    Json(#[from] serde_json::Error),
    #[error("invalid segments array")]
    InvalidSegments,
}

#[derive(Debug, Clone)]
pub struct MotionClip {
    pub duration: f32,
    pub fps: f32,
    pub fade_in_time: f32,
    pub fade_out_time: f32,
    pub looped: bool,
    pub are_beziers_restricted: bool,
    pub curves: Vec<Curve>,
    pub events: Vec<MotionEvent>,
}

impl MotionClip {
    /// Parse a motion (`.motion3.json`) document into an in-memory clip.
    pub fn parse(json_text: &str) -> Result<Self, MotionError> {
        let json: Motion3Json = serde_json::from_str(json_text)?;
        let fade_in_time = if json.meta.fade_in_time < 0.0 {
            1.0
        } else {
            json.meta.fade_in_time
        };
        let fade_out_time = if json.meta.fade_out_time < 0.0 {
            1.0
        } else {
            json.meta.fade_out_time
        };
        Ok(Self {
            duration: json.meta.duration,
            fps: json.meta.fps,
            fade_in_time,
            fade_out_time,
            looped: json.meta.looped,
            are_beziers_restricted: json.meta.are_beziers_restricted,
            curves: json
                .curves
                .into_iter()
                .map(|c| Curve::from_json(c, json.meta.are_beziers_restricted))
                .collect::<Result<Vec<_>, _>>()?,
            events: json
                .user_data
                .unwrap_or_default()
                .into_iter()
                .map(|e| MotionEvent {
                    time: e.time,
                    value: e.value,
                })
                .collect(),
        })
    }

    /// Compute the clip-level fade weight at `time` (taking looping into account).
    pub fn fade_weight(&self, time: f32) -> f32 {
        let t = if self.looped && self.duration > 0.0 {
            time.rem_euclid(self.duration)
        } else {
            time
        };

        let in_w = if self.fade_in_time > 0.0 {
            param_ops::easing_sine01((t / self.fade_in_time).clamp(0.0, 1.0))
        } else {
            1.0
        };

        let out_w = if self.looped {
            1.0
        } else if self.fade_out_time > 0.0 && self.duration > 0.0 {
            param_ops::easing_sine01(((self.duration - t) / self.fade_out_time).clamp(0.0, 1.0))
        } else {
            1.0
        };

        in_w * out_w
    }

    /// Sample the parameter curve for `id` at `time` (after loop normalization).
    pub fn sample_parameter_value(&self, id: &str, time: f32) -> Option<f32> {
        let t = if self.looped && self.duration > 0.0 {
            time.rem_euclid(self.duration)
        } else {
            time
        };
        self.curves
            .iter()
            .find(|c| c.target == CurveTarget::Parameter && c.id == id)
            .map(|c| c.evaluate(t))
    }
}

#[derive(Debug, Clone)]
pub struct MotionEvent {
    pub time: f32,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurveTarget {
    Model,
    Parameter,
    PartOpacity,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Curve {
    pub target: CurveTarget,
    pub id: String,
    pub fade_in_time: Option<f32>,
    pub fade_out_time: Option<f32>,
    segments: Vec<Segment>,
}

impl Curve {
    /// Convert the JSON curve representation into an evaluatable curve.
    fn from_json(j: CurveJson, are_beziers_restricted: bool) -> Result<Self, MotionError> {
        let target = match j.target.as_str() {
            "Model" => CurveTarget::Model,
            "Parameter" => CurveTarget::Parameter,
            "PartOpacity" => CurveTarget::PartOpacity,
            _ => CurveTarget::Unknown,
        };
        let bezier_mode = if are_beziers_restricted {
            BezierMode::Restricted
        } else {
            BezierMode::Cardano
        };
        let segments = parse_segments(&j.segments, bezier_mode)?;
        Ok(Self {
            target,
            id: j.id,
            fade_in_time: j.fade_in_time,
            fade_out_time: j.fade_out_time,
            segments,
        })
    }

    /// Evaluate the curve at `time`.
    pub fn evaluate(&self, time: f32) -> f32 {
        if self.segments.is_empty() {
            return 0.0;
        }

        for seg in &self.segments {
            if time <= seg.end_time() {
                return seg.evaluate(time);
            }
        }

        self.segments.last().map(|s| s.end_value()).unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
enum Segment {
    Linear {
        t0: f32,
        v0: f32,
        t1: f32,
        v1: f32,
    },
    Bezier {
        mode: BezierMode,
        t0: f32,
        v0: f32,
        c1t: f32,
        c1v: f32,
        c2t: f32,
        c2v: f32,
        t1: f32,
        v1: f32,
    },
    Stepped {
        v0: f32,
        t1: f32,
        v1: f32,
    },
    InverseStepped {
        v0: f32,
        t1: f32,
        v1: f32,
    },
}

impl Segment {
    /// Return the end time for this segment.
    fn end_time(&self) -> f32 {
        match *self {
            Segment::Linear { t1, .. }
            | Segment::Bezier { t1, .. }
            | Segment::Stepped { t1, .. }
            | Segment::InverseStepped { t1, .. } => t1,
        }
    }

    /// Return the end value for this segment.
    fn end_value(&self) -> f32 {
        match *self {
            Segment::Linear { v1, .. }
            | Segment::Bezier { v1, .. }
            | Segment::Stepped { v1, .. }
            | Segment::InverseStepped { v1, .. } => v1,
        }
    }

    /// Evaluate the segment at `time`.
    fn evaluate(&self, time: f32) -> f32 {
        match *self {
            Segment::Linear { t0, v0, t1, v1 } => {
                if t1 <= t0 {
                    return v1;
                }
                let u = ((time - t0) / (t1 - t0)).clamp(0.0, 1.0);
                v0 + (v1 - v0) * u
            }
            Segment::Bezier {
                mode,
                t0,
                v0,
                c1t,
                c1v,
                c2t,
                c2v,
                t1,
                v1,
            } => evaluate_bezier(mode, t0, v0, c1t, c1v, c2t, c2v, t1, v1, time),
            Segment::Stepped { v0, t1, v1, .. } => {
                if time < t1 {
                    v0
                } else {
                    v1
                }
            }
            Segment::InverseStepped { v0, t1, v1, .. } => {
                if time < t1 {
                    v1
                } else {
                    v0
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum BezierMode {
    Restricted,
    Cardano,
}

/// Parse the packed segments array from a motion3 JSON curve.
fn parse_segments(segments: &[f32], bezier_mode: BezierMode) -> Result<Vec<Segment>, MotionError> {
    if segments.len() < 2 {
        return Err(MotionError::InvalidSegments);
    }

    let mut out = Vec::new();
    let mut pos = 0usize;

    let mut t0 = segments[pos];
    let mut v0 = segments[pos + 1];
    pos += 2;

    while pos < segments.len() {
        let seg_type = segments
            .get(pos)
            .copied()
            .ok_or(MotionError::InvalidSegments)? as i32;
        pos += 1;

        match seg_type {
            0 => {
                let t1 = *segments.get(pos).ok_or(MotionError::InvalidSegments)?;
                let v1 = *segments.get(pos + 1).ok_or(MotionError::InvalidSegments)?;
                pos += 2;
                out.push(Segment::Linear { t0, v0, t1, v1 });
                t0 = t1;
                v0 = v1;
            }
            1 => {
                let c1t = *segments.get(pos).ok_or(MotionError::InvalidSegments)?;
                let c1v = *segments.get(pos + 1).ok_or(MotionError::InvalidSegments)?;
                let c2t = *segments.get(pos + 2).ok_or(MotionError::InvalidSegments)?;
                let c2v = *segments.get(pos + 3).ok_or(MotionError::InvalidSegments)?;
                let t1 = *segments.get(pos + 4).ok_or(MotionError::InvalidSegments)?;
                let v1 = *segments.get(pos + 5).ok_or(MotionError::InvalidSegments)?;
                pos += 6;
                out.push(Segment::Bezier {
                    mode: bezier_mode,
                    t0,
                    v0,
                    c1t,
                    c1v,
                    c2t,
                    c2v,
                    t1,
                    v1,
                });
                t0 = t1;
                v0 = v1;
            }
            2 => {
                let t1 = *segments.get(pos).ok_or(MotionError::InvalidSegments)?;
                let v1 = *segments.get(pos + 1).ok_or(MotionError::InvalidSegments)?;
                pos += 2;
                out.push(Segment::Stepped { v0, t1, v1 });
                t0 = t1;
                v0 = v1;
            }
            3 => {
                let t1 = *segments.get(pos).ok_or(MotionError::InvalidSegments)?;
                let v1 = *segments.get(pos + 1).ok_or(MotionError::InvalidSegments)?;
                pos += 2;
                out.push(Segment::InverseStepped { v0, t1, v1 });
                t0 = t1;
                v0 = v1;
            }
            _ => return Err(MotionError::InvalidSegments),
        }
    }

    Ok(out)
}

/// Evaluate a cubic Bezier segment at `time` using the selected solving strategy.
fn evaluate_bezier(
    mode: BezierMode,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    time: f32,
) -> f32 {
    match mode {
        BezierMode::Restricted => {
            let denom = x3 - x0;
            if denom.abs() <= f32::EPSILON {
                return y3;
            }
            let mut t = (time - x0) / denom;
            if t < 0.0 {
                t = 0.0;
            }
            de_casteljau_value(x0, y0, x1, y1, x2, y2, x3, y3, t)
        }
        BezierMode::Cardano => {
            let x = time;
            let a = x3 - 3.0 * x2 + 3.0 * x1 - x0;
            let b = 3.0 * x2 - 6.0 * x1 + 3.0 * x0;
            let c = 3.0 * x1 - 3.0 * x0;
            let d = x0 - x;
            let t = cardano_algorithm_for_bezier(a, b, c, d);
            de_casteljau_value(x0, y0, x1, y1, x2, y2, x3, y3, t)
        }
    }
}

/// Evaluate a cubic Bezier value using De Casteljau's algorithm.
fn de_casteljau_value(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    x3: f32,
    y3: f32,
    t: f32,
) -> f32 {
    let (p01x, p01y) = lerp_point(x0, y0, x1, y1, t);
    let (p12x, p12y) = lerp_point(x1, y1, x2, y2, t);
    let (p23x, p23y) = lerp_point(x2, y2, x3, y3, t);

    let (p012x, p012y) = lerp_point(p01x, p01y, p12x, p12y, t);
    let (p123x, p123y) = lerp_point(p12x, p12y, p23x, p23y, t);

    let (_p0123x, p0123y) = lerp_point(p012x, p012y, p123x, p123y, t);
    p0123y
}

/// Linear interpolation between two 2D points.
fn lerp_point(ax: f32, ay: f32, bx: f32, by: f32, t: f32) -> (f32, f32) {
    (ax + ((bx - ax) * t), ay + ((by - ay) * t))
}

/// Solve a quadratic equation and return one root.
fn quadratic_equation(a: f32, b: f32, c: f32) -> f32 {
    const EPS: f32 = 1e-6;
    if a.abs() < EPS {
        if b.abs() < EPS {
            return -c;
        }
        return -c / b;
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return 0.0;
    }
    -(b + disc.sqrt()) / (2.0 * a)
}

/// Solve for `t` in a cubic Bezier curve using Cardano's method.
fn cardano_algorithm_for_bezier(a: f32, b: f32, c: f32, d: f32) -> f32 {
    const EPS: f32 = 1e-6;
    const PI: f32 = core::f32::consts::PI;

    if a.abs() < EPS {
        return quadratic_equation(b, c, d).clamp(0.0, 1.0);
    }

    let ba = b / a;
    let ca = c / a;
    let da = d / a;

    let p = (3.0 * ca - ba * ba) / 3.0;
    let p3 = p / 3.0;
    let q = (2.0 * ba * ba * ba - 9.0 * ba * ca + 27.0 * da) / 27.0;
    let q2 = q / 2.0;
    let discriminant = q2 * q2 + p3 * p3 * p3;

    let center = 0.5f32;
    let threshold = center + 0.01f32;

    if discriminant < 0.0 {
        let mp3 = -p / 3.0;
        let mp33 = mp3 * mp3 * mp3;
        let r = mp33.sqrt();
        let t = -q / (2.0 * r);
        let cosphi = t.clamp(-1.0, 1.0);
        let phi = cosphi.acos();
        let crtr = r.cbrt();
        let t1 = 2.0 * crtr;

        let root1 = t1 * (phi / 3.0).cos() - ba / 3.0;
        if (root1 - center).abs() < threshold {
            return root1.clamp(0.0, 1.0);
        }

        let root2 = t1 * ((phi + 2.0 * PI) / 3.0).cos() - ba / 3.0;
        if (root2 - center).abs() < threshold {
            return root2.clamp(0.0, 1.0);
        }

        let root3 = t1 * ((phi + 4.0 * PI) / 3.0).cos() - ba / 3.0;
        return root3.clamp(0.0, 1.0);
    }

    if discriminant == 0.0 {
        let u1 = if q2 < 0.0 { (-q2).cbrt() } else { -(q2).cbrt() };

        let root1 = 2.0 * u1 - ba / 3.0;
        if (root1 - center).abs() < threshold {
            return root1.clamp(0.0, 1.0);
        }

        let root2 = -u1 - ba / 3.0;
        return root2.clamp(0.0, 1.0);
    }

    let sd = discriminant.sqrt();
    let u1 = (sd - q2).cbrt();
    let v1 = (sd + q2).cbrt();
    let root1 = u1 - v1 - ba / 3.0;
    root1.clamp(0.0, 1.0)
}

#[derive(Debug, Deserialize)]
struct Motion3Json {
    #[serde(rename = "Meta")]
    meta: Meta,
    #[serde(rename = "Curves")]
    curves: Vec<CurveJson>,
    #[serde(default, rename = "UserData")]
    user_data: Option<Vec<MotionEventJson>>,
}

#[derive(Debug, Deserialize)]
struct Meta {
    #[serde(rename = "Duration")]
    duration: f32,
    #[serde(rename = "Fps")]
    fps: f32,
    #[serde(rename = "FadeInTime")]
    fade_in_time: f32,
    #[serde(rename = "FadeOutTime")]
    fade_out_time: f32,
    #[serde(rename = "Loop")]
    looped: bool,
    #[serde(rename = "AreBeziersRestricted")]
    are_beziers_restricted: bool,
}

#[derive(Debug, Deserialize)]
struct CurveJson {
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "FadeInTime")]
    fade_in_time: Option<f32>,
    #[serde(default, rename = "FadeOutTime")]
    fade_out_time: Option<f32>,
    #[serde(rename = "Segments")]
    segments: Vec<f32>,
}

#[derive(Debug, Deserialize)]
struct MotionEventJson {
    #[serde(rename = "Time")]
    time: f32,
    #[serde(rename = "Value")]
    value: String,
}
