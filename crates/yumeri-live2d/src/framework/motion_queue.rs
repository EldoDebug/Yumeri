use crate::framework::param_ops;
use crate::framework::motion::{CurveTarget, MotionClip};
use crate::framework::VirtualParameters;
use crate::core::Model;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionPriority {
    #[default]
    None,
    Idle,
    Normal,
    Force,
}

/// Map a priority into an ordering rank (higher is stronger).
pub(crate) fn motion_priority_rank(p: MotionPriority) -> u8 {
    match p {
        MotionPriority::None => 0,
        MotionPriority::Idle => 1,
        MotionPriority::Normal => 2,
        MotionPriority::Force => 3,
    }
}

#[derive(Debug, Default)]
pub struct MotionQueueManager {
    user_time_seconds: f32,
    entries: Vec<MotionEntry>,
    current_priority: MotionPriority,
    reserve_priority: MotionPriority,
}

#[derive(Debug)]
struct MotionEntry {
    clip: Arc<MotionClip>,
    start_time: f32,
    fade_in_start_time: f32,
    end_time: Option<f32>,
    last_event_check_time: f32,
    fade_out_seconds: f32,
    triggered_fade_out: bool,
    fade_out_start_time: Option<f32>,
    priority: MotionPriority,
}

impl MotionQueueManager {
    /// Return `true` when no motions are currently active.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return the priority of the currently playing motion (if any).
    pub fn current_priority(&self) -> MotionPriority {
        self.current_priority
    }

    /// Return the reserved priority for the next motion.
    pub fn reserve_priority(&self) -> MotionPriority {
        self.reserve_priority
    }

    /// Reserve a slot for a motion with `priority`, returning `false` if it cannot preempt.
    pub fn reserve_motion(&mut self, priority: MotionPriority) -> bool {
        if motion_priority_rank(priority) <= motion_priority_rank(self.reserve_priority)
            || motion_priority_rank(priority) <= motion_priority_rank(self.current_priority)
        {
            return false;
        }
        self.reserve_priority = priority;
        true
    }

    /// Start playing `clip` at `priority`, fading out any existing entries.
    pub fn start_motion(&mut self, clip: Arc<MotionClip>, priority: MotionPriority) {
        if priority == self.reserve_priority {
            self.reserve_priority = MotionPriority::None;
        }
        self.current_priority = priority;

        for e in &mut self.entries {
            e.set_fade_out(e.clip.fade_out_time);
        }

        let start_time = self.user_time_seconds;
        let end_time = if clip.looped || clip.duration <= 0.0 {
            None
        } else {
            Some(start_time + clip.duration)
        };
        let fade_out_seconds = clip.fade_out_time.max(0.0);

        self.entries.push(MotionEntry {
            clip,
            start_time,
            fade_in_start_time: self.user_time_seconds,
            end_time,
            last_event_check_time: start_time,
            fade_out_seconds,
            triggered_fade_out: false,
            fade_out_start_time: None,
            priority,
        });
    }

    /// Return the maximum priority among active entries.
    pub fn current_max_priority(&self) -> Option<MotionPriority> {
        self.entries
            .iter()
            .map(|e| e.priority)
            .max_by_key(|p| motion_priority_rank(*p))
    }

    /// Return `true` if any active motion affects the given parameter id.
    pub fn affects_parameter_id(&self, id: &str) -> bool {
        self.entries.iter().any(|e| {
            e.clip.curves.iter().any(|c| {
                matches!(c.target, CurveTarget::Parameter | CurveTarget::PartOpacity) && c.id == id
            })
        })
    }

    /// Return `true` if any active motion contains the named model curve.
    pub fn affects_model_curve(&self, id: &str) -> bool {
        self.entries.iter().any(|e| {
            e.clip
                .curves
                .iter()
                .any(|c| c.target == CurveTarget::Model && c.id == id)
        })
    }

    /// Advance timers, apply curves to the model, and collect any fired events.
    pub fn update(
        &mut self,
        model: &mut Model,
        delta_time_seconds: f32,
        virtual_params: &mut VirtualParameters,
        out_model_opacity: &mut f32,
        param_index: &HashMap<String, usize>,
        _part_index: &HashMap<String, usize>,
        eye_blink_parameter_ids: &[String],
        lip_sync_parameter_ids: &[String],
        out_events: &mut Vec<String>,
    ) -> Result<(), crate::core::Error> {
        self.user_time_seconds += delta_time_seconds;

        for e in &mut self.entries {
            if e.triggered_fade_out && e.fade_out_start_time.is_none() {
                e.start_fade_out(self.user_time_seconds);
            }
        }

        {
            let mut params = model.parameters()?;
            for e in &mut self.entries {
                let local_time = e.local_time(self.user_time_seconds);
                let (global_fade_in, global_fade_out) = e.fade_weights(self.user_time_seconds);
                let global_weight = (global_fade_in * global_fade_out).clamp(0.0, 1.0);

                let mut eye_blink_value: Option<f32> = None;
                let mut lip_sync_value: Option<f32> = None;
                let mut opacity_value: Option<f32> = None;
                for curve in &e.clip.curves {
                    if curve.target != CurveTarget::Model {
                        continue;
                    }
                    match curve.id.as_str() {
                        "EyeBlink" => eye_blink_value = Some(curve.evaluate(local_time)),
                        "LipSync" => lip_sync_value = Some(curve.evaluate(local_time)),
                        "Opacity" => opacity_value = Some(curve.evaluate(local_time)),
                        _ => {}
                    }
                }

                if let Some(v) = opacity_value {
                    *out_model_opacity = (*out_model_opacity * (1.0 - global_weight)
                        + v * global_weight)
                        .clamp(0.0, 1.0);
                }

                let mut eye_blink_overridden = vec![false; eye_blink_parameter_ids.len()];
                let mut lip_sync_overridden = vec![false; lip_sync_parameter_ids.len()];

                for curve in &e.clip.curves {
                    match curve.target {
                        CurveTarget::Model => {}
                        CurveTarget::Parameter => {
                            let w = if curve.fade_in_time.is_none() && curve.fade_out_time.is_none()
                            {
                                global_weight
                            } else {
                                curve_weight(
                                    e,
                                    self.user_time_seconds,
                                    global_fade_in,
                                    global_fade_out,
                                    curve,
                                )
                            };
                            if w <= 0.0 {
                                continue;
                            }
                            let mut v = curve.evaluate(local_time);

                            if let Some(ev) = eye_blink_value {
                                for (i, id) in eye_blink_parameter_ids.iter().enumerate() {
                                    if id == &curve.id {
                                        v *= ev;
                                        eye_blink_overridden[i] = true;
                                        break;
                                    }
                                }
                            }

                            if let Some(lv) = lip_sync_value {
                                for (i, id) in lip_sync_parameter_ids.iter().enumerate() {
                                    if id == &curve.id {
                                        v += lv;
                                        lip_sync_overridden[i] = true;
                                        break;
                                    }
                                }
                            }

                            if let Some(&idx) = param_index.get(&curve.id) {
                                param_ops::set_parameter_value(&mut params, idx, v, w);
                            } else {
                                let cur = virtual_params.get(&curve.id).copied().unwrap_or(0.0);
                                virtual_params.insert(curve.id.clone(), cur * (1.0 - w) + v * w);
                            }
                        }
                        CurveTarget::PartOpacity => {
                            let v = curve.evaluate(local_time);
                            if let Some(&idx) = param_index.get(&curve.id) {
                                param_ops::set_parameter_value(&mut params, idx, v, 1.0);
                            } else {
                                virtual_params.insert(curve.id.clone(), v.clamp(0.0, 1.0));
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(v) = eye_blink_value {
                    for (i, id) in eye_blink_parameter_ids.iter().enumerate() {
                        if eye_blink_overridden.get(i).copied().unwrap_or(false) {
                            continue;
                        }
                        if let Some(&idx) = param_index.get(id) {
                            param_ops::set_parameter_value(&mut params, idx, v, global_weight);
                        } else {
                            let cur = virtual_params.get(id).copied().unwrap_or(0.0);
                            virtual_params.insert(
                                id.clone(),
                                cur * (1.0 - global_weight) + v * global_weight,
                            );
                        }
                    }
                }

                if let Some(v) = lip_sync_value {
                    for (i, id) in lip_sync_parameter_ids.iter().enumerate() {
                        if lip_sync_overridden.get(i).copied().unwrap_or(false) {
                            continue;
                        }
                        if let Some(&idx) = param_index.get(id) {
                            param_ops::set_parameter_value(&mut params, idx, v, global_weight);
                        } else {
                            let cur = virtual_params.get(id).copied().unwrap_or(0.0);
                            virtual_params.insert(
                                id.clone(),
                                cur * (1.0 - global_weight) + v * global_weight,
                            );
                        }
                    }
                }

                e.fire_events(self.user_time_seconds, out_events);
                e.last_event_check_time = self.user_time_seconds;
            }
        }

        self.entries
            .retain(|e| !e.is_finished(self.user_time_seconds));
        if self.entries.is_empty() {
            self.current_priority = MotionPriority::None;
        }

        Ok(())
    }
}

impl MotionEntry {
    /// Mark the entry for fade-out using the given duration.
    fn set_fade_out(&mut self, fade_out_seconds: f32) {
        self.fade_out_seconds = fade_out_seconds.max(0.0);
        self.triggered_fade_out = true;
    }

    /// Start fading out at `user_time_seconds` and compute the entry end time.
    fn start_fade_out(&mut self, user_time_seconds: f32) {
        self.fade_out_start_time = Some(user_time_seconds);
        let new_end = user_time_seconds + self.fade_out_seconds;
        self.end_time = match self.end_time {
            Some(cur) if cur >= 0.0 && cur <= new_end => Some(cur),
            _ => Some(new_end),
        };
    }

    /// Return `true` when the entry has finished playing (including fade-out).
    fn is_finished(&self, user_time_seconds: f32) -> bool {
        self.end_time.is_some_and(|t| user_time_seconds >= t)
    }

    /// Convert global time into clip-local time, applying loop wrapping when enabled.
    fn local_time(&self, user_time_seconds: f32) -> f32 {
        let t = user_time_seconds - self.start_time;
        if self.clip.looped && self.clip.duration > 0.0 {
            t.rem_euclid(self.clip.duration)
        } else {
            t
        }
    }

    /// Compute (fade_in, fade_out) weights in `[0, 1]` for this entry at `user_time_seconds`.
    fn fade_weights(&self, user_time_seconds: f32) -> (f32, f32) {
        let fade_in = if self.clip.fade_in_time <= 0.0 {
            1.0
        } else {
            param_ops::easing_sine01(
                (user_time_seconds - self.fade_in_start_time) / self.clip.fade_in_time,
            )
        };

        let fade_out = if self.clip.looped {
            1.0
        } else if self.fade_out_seconds > 0.0 {
            match self.end_time {
                Some(end_time) => {
                    param_ops::easing_sine01((end_time - user_time_seconds) / self.fade_out_seconds)
                }
                None => 1.0,
            }
        } else {
            1.0
        };

        (fade_in, fade_out)
    }

    /// Append any newly-fired motion events to `out`.
    fn fire_events(&self, user_time_seconds: f32, out: &mut Vec<String>) {
        if self.clip.events.is_empty() {
            return;
        }
        let before = (self.last_event_check_time - self.start_time).max(0.0);
        let now = (user_time_seconds - self.start_time).max(0.0);

        if self.clip.looped && self.clip.duration > 0.0 {
            let d = self.clip.duration;
            let b = before.rem_euclid(d);
            let n = now.rem_euclid(d);
            if b < n {
                for e in &self.clip.events {
                    if e.time > b && e.time <= n {
                        out.push(e.value.clone());
                    }
                }
            } else if b > n {
                for e in &self.clip.events {
                    if e.time > b && e.time <= d {
                        out.push(e.value.clone());
                    } else if e.time >= 0.0 && e.time <= n {
                        out.push(e.value.clone());
                    }
                }
            }
        } else {
            for e in &self.clip.events {
                if e.time > before && e.time <= now {
                    out.push(e.value.clone());
                }
            }
        }
    }
}

/// Compute the per-curve fade weight for `entry` at `user_time_seconds`.
fn curve_weight(
    entry: &MotionEntry,
    user_time_seconds: f32,
    global_fade_in: f32,
    global_fade_out: f32,
    curve: &super::motion::Curve,
) -> f32 {
    let fin = match curve.fade_in_time {
        None => global_fade_in,
        Some(t) if t <= 0.0 => 1.0,
        Some(t) => param_ops::easing_sine01((user_time_seconds - entry.fade_in_start_time) / t),
    };

    let fout = match curve.fade_out_time {
        None => global_fade_out,
        Some(t) if t <= 0.0 => 1.0,
        Some(t) => {
            if entry.clip.looped {
                1.0
            } else {
                match entry.end_time {
                    Some(end_time) => param_ops::easing_sine01((end_time - user_time_seconds) / t),
                    None => 1.0,
                }
            }
        }
    };

    (fin * fout).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Moc, Model};
    use std::collections::HashMap;

    #[test]
    #[ignore]
    fn part_opacity_updates_even_when_entry_weight_is_zero()
    -> Result<(), Box<dyn std::error::Error>> {
        let moc3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.moc3");
        let moc3 = std::fs::read(moc3_path)?;
        let moc = Moc::from_bytes(&moc3)?;
        let mut model = Model::new(moc)?;

        let motion_json = r#"
        {
          "Meta": {
            "Duration": 1.0,
            "Fps": 30.0,
            "FadeInTime": 0.0,
            "FadeOutTime": 0.5,
            "Loop": false,
            "AreBeziersRestricted": false
          },
          "Curves": [
            {
              "Target": "PartOpacity",
              "Id": "ParamAngleX",
              "Segments": [0.0, 0.0, 0, 1.0, 1.0]
            }
          ]
        }
        "#;
        let clip = Arc::new(MotionClip::parse(motion_json)?);

        let mut param_index = HashMap::new();
        {
            let params = model.parameters()?;
            for i in 0..params.len() {
                param_index.insert(params.id(i).to_string_lossy().into_owned(), i);
            }
        }

        let mut q = MotionQueueManager::default();
        q.start_motion(clip, MotionPriority::Normal);
        let mut model_opacity = 1.0f32;
        q.update(
            &mut model,
            1.0,
            &mut VirtualParameters::default(),
            &mut model_opacity,
            &param_index,
            &HashMap::new(),
            &[],
            &[],
            &mut Vec::new(),
        )?;

        let params = model.parameters()?;
        let &idx = param_index
            .get("ParamAngleX")
            .ok_or("ParamAngleX not found")?;
        assert!((params.values()[idx] - 1.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    #[ignore]
    fn parameter_curve_uses_global_fade_once() -> Result<(), Box<dyn std::error::Error>> {
        let moc3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.moc3");
        let moc3 = std::fs::read(moc3_path)?;
        let moc = Moc::from_bytes(&moc3)?;
        let mut model = Model::new(moc)?;

        let mut param_index = HashMap::new();
        {
            let params = model.parameters()?;
            for i in 0..params.len() {
                param_index.insert(params.id(i).to_string_lossy().into_owned(), i);
            }
        }

        let &idx = param_index
            .get("ParamAngleX")
            .ok_or("ParamAngleX not found")?;
        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }

        let motion_json = r#"
        {
          "Meta": {
            "Duration": 2.0,
            "Fps": 30.0,
            "FadeInTime": 1.0,
            "FadeOutTime": 1.0,
            "Loop": false,
            "AreBeziersRestricted": false
          },
          "Curves": [
            {
              "Target": "Parameter",
              "Id": "ParamAngleX",
              "Segments": [0.0, 1.0, 0, 1.0, 1.0]
            }
          ]
        }
        "#;
        let clip = Arc::new(MotionClip::parse(motion_json)?);

        let mut q = MotionQueueManager::default();
        q.start_motion(clip, MotionPriority::Normal);
        let mut model_opacity = 1.0f32;
        q.update(
            &mut model,
            0.5,
            &mut VirtualParameters::default(),
            &mut model_opacity,
            &param_index,
            &HashMap::new(),
            &[],
            &[],
            &mut Vec::new(),
        )?;

        let params = model.parameters()?;
        let v = params.values()[idx];
        assert!((v - 0.5).abs() < 1e-6, "ParamAngleX={v} (expected 0.5)");
        Ok(())
    }

    #[test]
    #[ignore]
    fn model_curve_eye_blink_affects_parameter_list() -> Result<(), Box<dyn std::error::Error>> {
        let moc3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.moc3");
        let moc3 = std::fs::read(moc3_path)?;
        let moc = Moc::from_bytes(&moc3)?;
        let mut model = Model::new(moc)?;

        let mut param_index = HashMap::new();
        {
            let params = model.parameters()?;
            for i in 0..params.len() {
                param_index.insert(params.id(i).to_string_lossy().into_owned(), i);
            }
        }
        let &idx = param_index
            .get("ParamAngleX")
            .ok_or("ParamAngleX not found")?;
        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }

        let motion_json = r#"
        {
          "Meta": {
            "Duration": 1.0,
            "Fps": 30.0,
            "FadeInTime": 0.0,
            "FadeOutTime": 0.0,
            "Loop": false,
            "AreBeziersRestricted": false
          },
          "Curves": [
            {
              "Target": "Model",
              "Id": "EyeBlink",
              "Segments": [0.0, 0.5, 0, 1.0, 0.5]
            },
            {
              "Target": "Parameter",
              "Id": "ParamAngleX",
              "Segments": [0.0, 1.0, 0, 1.0, 1.0]
            }
          ]
        }
        "#;
        let clip = Arc::new(MotionClip::parse(motion_json)?);

        let mut q = MotionQueueManager::default();
        q.start_motion(clip, MotionPriority::Normal);
        let mut model_opacity = 1.0f32;
        q.update(
            &mut model,
            0.0,
            &mut VirtualParameters::default(),
            &mut model_opacity,
            &param_index,
            &HashMap::new(),
            &[String::from("ParamAngleX")],
            &[],
            &mut Vec::new(),
        )?;

        let params = model.parameters()?;
        let v = params.values()[idx];
        assert!((v - 0.5).abs() < 1e-6, "ParamAngleX={v} (expected 0.5)");
        Ok(())
    }

    #[test]
    #[ignore]
    fn model_curve_opacity_updates_model_opacity() -> Result<(), Box<dyn std::error::Error>> {
        let moc3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.moc3");
        let moc3 = std::fs::read(moc3_path)?;
        let moc = Moc::from_bytes(&moc3)?;
        let mut model = Model::new(moc)?;

        let mut param_index = HashMap::new();
        {
            let params = model.parameters()?;
            for i in 0..params.len() {
                param_index.insert(params.id(i).to_string_lossy().into_owned(), i);
            }
        }

        let motion_json = r#"
        {
          "Meta": {
            "Duration": 1.0,
            "Fps": 30.0,
            "FadeInTime": 0.0,
            "FadeOutTime": 0.0,
            "Loop": false,
            "AreBeziersRestricted": false
          },
          "Curves": [
            {
              "Target": "Model",
              "Id": "Opacity",
              "Segments": [0.0, 0.25, 0, 1.0, 0.25]
            }
          ]
        }
        "#;
        let clip = Arc::new(MotionClip::parse(motion_json)?);

        let mut q = MotionQueueManager::default();
        q.start_motion(clip, MotionPriority::Normal);
        let mut model_opacity = 1.0f32;
        q.update(
            &mut model,
            0.0,
            &mut VirtualParameters::default(),
            &mut model_opacity,
            &param_index,
            &HashMap::new(),
            &[],
            &[],
            &mut Vec::new(),
        )?;

        assert!(
            (model_opacity - 0.25).abs() < 1e-6,
            "model_opacity={model_opacity}"
        );
        Ok(())
    }
}
