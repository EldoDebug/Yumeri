use crate::core::Model;
use crate::framework::param_ops;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Expression data (from exp3.json)
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ExpressionError {
    #[error("failed to parse exp3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub fade_in_time: f32,
    pub fade_out_time: f32,
    pub parameters: Vec<ExpressionParameter>,
}

impl Expression {
    /// Parse an expression (`.exp3.json`) document.
    pub fn parse(json_text: &str) -> Result<Self, ExpressionError> {
        let json: Exp3Json = serde_json::from_str(json_text)?;
        Ok(Self {
            fade_in_time: json.fade_in_time,
            fade_out_time: json.fade_out_time,
            parameters: json
                .parameters
                .into_iter()
                .map(|p| ExpressionParameter {
                    id: p.id,
                    value: p.value,
                    blend: match p.blend.as_str() {
                        "Add" => ExpressionBlend::Add,
                        "Multiply" => ExpressionBlend::Multiply,
                        _ => ExpressionBlend::Overwrite,
                    },
                })
                .collect(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExpressionParameter {
    pub id: String,
    pub value: f32,
    pub blend: ExpressionBlend,
}

#[derive(Debug, Clone, Copy)]
pub enum ExpressionBlend {
    Add,
    Multiply,
    Overwrite,
}

#[derive(Debug, Deserialize)]
struct Exp3Json {
    #[serde(default, rename = "FadeInTime")]
    fade_in_time: f32,
    #[serde(default, rename = "FadeOutTime")]
    fade_out_time: f32,
    #[serde(default, rename = "Parameters")]
    parameters: Vec<Exp3Param>,
}

#[derive(Debug, Deserialize)]
struct Exp3Param {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: f32,
    #[serde(rename = "Blend")]
    blend: String,
}

// ---------------------------------------------------------------------------
// Expression manager
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct ExpressionManager {
    user_time_seconds: f32,
    entries: Vec<ExpressionEntry>,
}

#[derive(Debug)]
struct ExpressionEntry {
    expr: Arc<Expression>,
    fade_in_start_time: f32,
    end_time: Option<f32>,
    fade_out_seconds: f32,
    triggered_fade_out: bool,
    fade_out_start_time: Option<f32>,
}

impl ExpressionManager {
    /// Return `true` when no expressions are currently active.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Start (or stack) an expression, fading out any currently active ones.
    pub fn start_expression(&mut self, expr: Arc<Expression>) {
        for e in &mut self.entries {
            e.set_fade_out(e.expr.fade_out_time);
        }

        let start_time = self.user_time_seconds;
        self.entries.push(ExpressionEntry {
            expr,
            fade_in_start_time: start_time,
            end_time: None,
            fade_out_seconds: 0.0,
            triggered_fade_out: false,
            fade_out_start_time: None,
        });
    }

    /// Return `true` if any active expression references the given parameter id.
    pub fn affects_parameter_id(&self, id: &str) -> bool {
        self.entries
            .iter()
            .any(|e| e.expr.parameters.iter().any(|p| p.id == id))
    }

    /// Advance expression timers and apply the blended result to `model` parameters.
    pub fn update(
        &mut self,
        model: &mut Model,
        delta_time_seconds: f32,
        param_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        self.user_time_seconds += delta_time_seconds;

        for e in &mut self.entries {
            if e.triggered_fade_out && e.fade_out_start_time.is_none() {
                e.start_fade_out(self.user_time_seconds);
            }
        }

        if self.entries.is_empty() {
            return Ok(());
        }

        let mut params = model.parameters()?;

        let mut affected = Vec::<usize>::new();
        let mut seen = HashSet::<usize>::new();
        for e in &self.entries {
            for p in &e.expr.parameters {
                let Some(&idx) = param_index.get(&p.id) else {
                    continue;
                };
                if seen.insert(idx) {
                    affected.push(idx);
                }
            }
        }
        if affected.is_empty() {
            self.cleanup_entries();
            return Ok(());
        }

        let mut baseline = Vec::<f32>::with_capacity(affected.len());
        for &idx in &affected {
            baseline.push(params.values()[idx]);
        }

        let mut additive = vec![0.0f32; affected.len()];
        let mut multiply = vec![1.0f32; affected.len()];
        let mut overwrite = baseline.clone();

        for (expression_index, e) in self.entries.iter().enumerate() {
            let w = e.fade_weight(self.user_time_seconds).clamp(0.0, 1.0);

            let mut map = HashMap::<usize, (ExpressionBlend, f32)>::new();
            for p in &e.expr.parameters {
                let Some(&idx) = param_index.get(&p.id) else {
                    continue;
                };
                map.insert(idx, (p.blend, p.value));
            }

            for (i, &idx) in affected.iter().enumerate() {
                let cur = baseline[i];
                let (target_add, target_mul, target_over) = match map.get(&idx) {
                    None => (0.0, 1.0, cur),
                    Some((ExpressionBlend::Add, v)) => (*v, 1.0, cur),
                    Some((ExpressionBlend::Multiply, v)) => (0.0, *v, cur),
                    Some((ExpressionBlend::Overwrite, v)) => (0.0, 1.0, *v),
                };

                if expression_index == 0 {
                    additive[i] = target_add;
                    multiply[i] = target_mul;
                    overwrite[i] = target_over;
                } else {
                    additive[i] = lerp(additive[i], target_add, w);
                    multiply[i] = lerp(multiply[i], target_mul, w);
                    overwrite[i] = lerp(overwrite[i], target_over, w);
                }
            }
        }

        let mut expression_weight = 0.0f32;
        for e in &self.entries {
            expression_weight += e.fade_in_weight(self.user_time_seconds);
        }
        expression_weight = expression_weight.clamp(0.0, 1.0);
        if expression_weight > 0.0 {
            for (i, &idx) in affected.iter().enumerate() {
                let value = (overwrite[i] + additive[i]) * multiply[i];
                param_ops::set_parameter_value(&mut params, idx, value, expression_weight);
            }
        }

        self.cleanup_entries();

        Ok(())
    }

    /// Drop finished entries and collapse to the last fully-applied expression.
    fn cleanup_entries(&mut self) {
        if self.entries.len() > 1 {
            if let Some(last) = self.entries.last() {
                if last.fade_weight(self.user_time_seconds) >= 1.0 {
                    let last = self.entries.pop().expect("len>1");
                    self.entries.clear();
                    self.entries.push(last);
                }
            }
        }
        self.entries
            .retain(|e| !e.is_finished(self.user_time_seconds));
    }
}

impl ExpressionEntry {
    /// Mark the entry for fade-out using the given duration.
    fn set_fade_out(&mut self, fade_out_seconds: f32) {
        self.fade_out_seconds = fade_out_seconds.max(0.0);
        self.triggered_fade_out = true;
    }

    /// Start fading out at `user_time_seconds` and compute the end time.
    fn start_fade_out(&mut self, user_time_seconds: f32) {
        self.fade_out_start_time = Some(user_time_seconds);
        let new_end = user_time_seconds + self.fade_out_seconds;
        self.end_time = match self.end_time {
            Some(cur) if cur >= 0.0 && cur <= new_end => Some(cur),
            _ => Some(new_end),
        };
    }

    /// Return `true` when this entry has reached its end time.
    fn is_finished(&self, user_time_seconds: f32) -> bool {
        self.end_time.is_some_and(|t| user_time_seconds >= t)
    }

    /// Return the fade-in weight in `[0, 1]` at `user_time_seconds`.
    fn fade_in_weight(&self, user_time_seconds: f32) -> f32 {
        if self.expr.fade_in_time <= 0.0 {
            return 1.0;
        }
        param_ops::easing_sine01(
            (user_time_seconds - self.fade_in_start_time) / self.expr.fade_in_time,
        )
        .clamp(0.0, 1.0)
    }

    /// Return the combined fade-in/out weight in `[0, 1]` at `user_time_seconds`.
    fn fade_weight(&self, user_time_seconds: f32) -> f32 {
        let fade_in = if self.expr.fade_in_time <= 0.0 {
            1.0
        } else {
            param_ops::easing_sine01(
                (user_time_seconds - self.fade_in_start_time) / self.expr.fade_in_time,
            )
        };

        let fade_out = if self.fade_out_seconds > 0.0 {
            match self.end_time {
                Some(end_time) => {
                    param_ops::easing_sine01((end_time - user_time_seconds) / self.fade_out_seconds)
                }
                None => 1.0,
            }
        } else {
            1.0
        };

        (fade_in * fade_out).clamp(0.0, 1.0)
    }
}

/// Linear interpolation between `a` and `b`.
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + b * t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Moc, Model};

    fn mao_model() -> anyhow::Result<(Model, HashMap<String, usize>)> {
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
        Ok((model, param_index))
    }

    fn make_expression(
        fade_in: f32,
        fade_out: f32,
        params: Vec<ExpressionParameter>,
    ) -> Expression {
        Expression {
            fade_in_time: fade_in,
            fade_out_time: fade_out,
            parameters: params,
        }
    }

    #[test]
    #[ignore]
    fn expression_overwrite_fades_in_like_native() -> anyhow::Result<()> {
        let (mut model, param_index) = mao_model()?;
        let idx = *param_index.get("ParamAngleX").expect("ParamAngleX exists");

        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }

        let expr = Arc::new(make_expression(
            1.0,
            1.0,
            vec![ExpressionParameter {
                id: "ParamAngleX".to_string(),
                value: 2.0,
                blend: ExpressionBlend::Overwrite,
            }],
        ));

        let mut mgr = ExpressionManager::default();
        mgr.start_expression(expr);

        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }
        mgr.update(&mut model, 0.5, &param_index)?;
        let v = { model.parameters()?.values()[idx] };
        assert!((v - 1.0).abs() < 1e-4, "ParamAngleX={v} (expected ~1.0)");
        Ok(())
    }

    #[test]
    #[ignore]
    fn expression_crossfade_matches_native_combination() -> anyhow::Result<()> {
        let (mut model, param_index) = mao_model()?;
        let idx = *param_index.get("ParamAngleX").expect("ParamAngleX exists");

        let expr_old = Arc::new(make_expression(
            0.0,
            1.0,
            vec![ExpressionParameter {
                id: "ParamAngleX".to_string(),
                value: 2.0,
                blend: ExpressionBlend::Overwrite,
            }],
        ));
        let expr_new = Arc::new(make_expression(
            1.0,
            1.0,
            vec![ExpressionParameter {
                id: "ParamAngleX".to_string(),
                value: 1.0,
                blend: ExpressionBlend::Add,
            }],
        ));

        let mut mgr = ExpressionManager::default();
        mgr.start_expression(expr_old);

        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }
        mgr.update(&mut model, 0.0, &param_index)?;
        assert!((model.parameters()?.values()[idx] - 2.0).abs() < 1e-4);

        mgr.start_expression(expr_new);

        {
            let mut params = model.parameters()?;
            params.values_mut()[idx] = 0.0;
        }
        mgr.update(&mut model, 0.5, &param_index)?;
        let v = { model.parameters()?.values()[idx] };
        assert!((v - 1.5).abs() < 1e-3, "ParamAngleX={v} (expected ~1.5)");

        Ok(())
    }
}
