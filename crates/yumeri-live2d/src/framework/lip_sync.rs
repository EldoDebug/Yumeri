use crate::core::Model;
use crate::framework::param_ops;

#[derive(Debug, Default, Clone, Copy)]
pub struct LipSync {
    value_0_to_1: f32,
}

impl LipSync {
    /// Set the normalized lip-sync value in the `[0, 1]` range.
    pub fn set_value(&mut self, value_0_to_1: f32) {
        self.value_0_to_1 = value_0_to_1.clamp(0.0, 1.0);
    }

    /// Return the current normalized lip-sync value.
    pub fn value(&self) -> f32 {
        self.value_0_to_1
    }

    /// Apply the current lip-sync value to the model parameters referenced by `lip_sync_param_indices`.
    pub fn apply(
        &self,
        model: &mut Model,
        lip_sync_param_indices: &[usize],
        weight: f32,
    ) -> Result<(), crate::core::Error> {
        if lip_sync_param_indices.is_empty() {
            return Ok(());
        }
        let v = self.value_0_to_1;
        if v <= 0.0 {
            return Ok(());
        }
        let mut params = model.parameters()?;
        for &idx in lip_sync_param_indices {
            param_ops::add_parameter_value(&mut params, idx, v, weight);
        }
        Ok(())
    }
}
