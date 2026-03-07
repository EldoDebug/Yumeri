use crate::core::Model;
use crate::framework::param_ops;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BreathParameter {
    pub id: String,
    pub offset: f32,
    pub peak: f32,
    pub cycle: f32,
    pub weight: f32,
}

#[derive(Debug, Default)]
pub struct Breath {
    params: Vec<BreathParameter>,
    current_time: f32,
}

impl Breath {
    /// Replace the set of parameters affected by this breath controller.
    pub fn set_parameters(&mut self, params: Vec<BreathParameter>) {
        self.params = params;
    }

    /// Return `true` when at least one breath parameter is configured.
    pub fn is_enabled(&self) -> bool {
        !self.params.is_empty()
    }

    /// Advance the breath phase and apply parameter deltas to the model.
    pub fn update(
        &mut self,
        model: &mut Model,
        delta_time_seconds: f32,
        param_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        if self.params.is_empty() || delta_time_seconds <= 0.0 {
            return Ok(());
        }

        self.current_time += delta_time_seconds;
        let t = self.current_time * 2.0 * core::f32::consts::PI;

        let mut params = model.parameters()?;
        for p in &self.params {
            let Some(&idx) = param_index.get(&p.id) else {
                continue;
            };
            if p.cycle.abs() <= f32::EPSILON {
                continue;
            }
            let v = p.offset + p.peak * (t / p.cycle).sin();
            param_ops::add_parameter_value(&mut params, idx, v, p.weight);
        }

        Ok(())
    }
}
