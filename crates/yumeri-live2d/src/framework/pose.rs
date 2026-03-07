use crate::core::Model;
use crate::framework::param_ops;
use crate::framework::VirtualParameters;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum PoseError {
    #[error("failed to parse pose3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct Pose {
    fade_time_seconds: f32,
    groups: Vec<Vec<PosePart>>,
    last_model_ptr: Option<usize>,
}

#[derive(Debug, Clone)]
struct PosePart {
    id: String,
    links: Vec<String>,
    parameter_index: Option<usize>,
    part_index: Option<usize>,
    link_part_indices: Vec<usize>,
}

impl Pose {
    /// Parse a pose (`.pose3.json`) document.
    pub fn parse(json_text: &str) -> Result<Self, PoseError> {
        let json: Pose3Json = serde_json::from_str(json_text)?;
        let fade_time_seconds = json.fade_in_time.unwrap_or(0.5).max(0.0);
        let groups = json
            .groups
            .into_iter()
            .map(|g| {
                g.into_iter()
                    .map(|p| PosePart {
                        id: p.id,
                        links: p.link,
                        parameter_index: None,
                        part_index: None,
                        link_part_indices: Vec::new(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        Ok(Self {
            fade_time_seconds,
            groups,
            last_model_ptr: None,
        })
    }

    /// Resolve parameter/part indices using the provided id-to-index maps.
    pub fn resolve_indices(
        &mut self,
        param_index: &HashMap<String, usize>,
        part_index: &HashMap<String, usize>,
    ) {
        for g in &mut self.groups {
            for p in g {
                p.parameter_index = param_index.get(&p.id).copied();
                p.part_index = part_index.get(&p.id).copied();
                p.link_part_indices = p
                    .links
                    .iter()
                    .filter_map(|id| part_index.get(id).copied())
                    .collect::<Vec<_>>();
            }
        }
    }

    /// Reset pose state and apply initial visibility/opacities to the model.
    pub fn reset(
        &mut self,
        model: &mut Model,
        virtual_params: &mut VirtualParameters,
        param_index: &HashMap<String, usize>,
        part_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        self.resolve_indices(param_index, part_index);

        {
            let mut params = model.parameters()?;
            for g in &mut self.groups {
                for p in g.iter_mut() {
                    if let Some(idx) = p.parameter_index {
                        param_ops::set_parameter_value(&mut params, idx, 1.0, 1.0);
                    } else {
                        virtual_params.insert(p.id.clone(), 1.0);
                    }
                }
            }
        }

        {
            let mut parts = model.parts()?;
            let opacities = parts.opacities_mut();
            for g in &mut self.groups {
                for (i, p) in g.iter_mut().enumerate() {
                    let visible = i == 0;
                    if let Some(part_idx) = p.part_index {
                        param_ops::set_part_opacity(
                            opacities,
                            part_idx,
                            if visible { 1.0 } else { 0.0 },
                        );
                    }
                }
            }
        }

        {
            let mut params = model.parameters()?;
            for g in &mut self.groups {
                for (i, p) in g.iter_mut().enumerate() {
                    let visible = i == 0;
                    if let Some(param_idx) = p.parameter_index {
                        param_ops::set_parameter_value(
                            &mut params,
                            param_idx,
                            if visible { 1.0 } else { 0.0 },
                            1.0,
                        );
                    } else {
                        virtual_params.insert(p.id.clone(), if visible { 1.0 } else { 0.0 });
                    }
                }
            }
        }

        self.last_model_ptr = Some(model.as_ptr() as usize);
        Ok(())
    }

    /// Update part opacities for the current frame and propagate linked parts.
    pub fn update_parameters(
        &mut self,
        model: &mut Model,
        virtual_params: &mut VirtualParameters,
        delta_time_seconds: f32,
        param_index: &HashMap<String, usize>,
        part_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        let model_ptr = model.as_ptr() as usize;
        if self.last_model_ptr != Some(model_ptr) {
            self.reset(model, virtual_params, param_index, part_index)?;
        }

        let mut dt = delta_time_seconds;
        if dt < 0.0 {
            dt = 0.0;
        }

        const EPSILON: f32 = 0.001;
        const PHI: f32 = 0.5;
        const BACK_OPACITY_THRESHOLD: f32 = 0.15;

        for g in &self.groups {
            if g.is_empty() {
                continue;
            }

            let visible_idx = {
                let params = model.parameters()?;
                let mut visible: Option<usize> = None;
                for (i, p) in g.iter().enumerate() {
                    let v = if let Some(param_idx) = p.parameter_index {
                        params.values()[param_idx]
                    } else {
                        virtual_params.get(&p.id).copied().unwrap_or(0.0)
                    };
                    if v > EPSILON {
                        if visible.is_some() {
                            break;
                        }
                        visible = Some(i);
                    }
                }
                visible.unwrap_or(0)
            };

            let mut new_opacity = 1.0f32;
            if self.fade_time_seconds > 0.0 {
                if let Some(part_idx) = g.get(visible_idx).and_then(|p| p.part_index) {
                    let mut parts = model.parts()?;
                    new_opacity = parts.opacities_mut()[part_idx];
                }
                new_opacity = (new_opacity + dt / self.fade_time_seconds).min(1.0);
            }

            {
                let mut parts = model.parts()?;
                let opacities = parts.opacities_mut();

                for (i, p) in g.iter().enumerate() {
                    let Some(part_idx) = p.part_index else {
                        continue;
                    };
                    if i == visible_idx {
                        param_ops::set_part_opacity(opacities, part_idx, new_opacity);
                        continue;
                    }

                    let mut opacity = opacities[part_idx];
                    let mut a1 = if new_opacity < PHI {
                        new_opacity * (PHI - 1.0) / PHI + 1.0
                    } else if (1.0 - PHI).abs() > f32::EPSILON {
                        (1.0 - new_opacity) * PHI / (1.0 - PHI)
                    } else {
                        1.0
                    };

                    let back_opacity = (1.0 - a1) * (1.0 - new_opacity);
                    if back_opacity > BACK_OPACITY_THRESHOLD
                        && (1.0 - new_opacity).abs() > f32::EPSILON
                    {
                        a1 = 1.0 - BACK_OPACITY_THRESHOLD / (1.0 - new_opacity);
                    }

                    if opacity > a1 {
                        opacity = a1;
                    }

                    param_ops::set_part_opacity(opacities, part_idx, opacity);
                }

                for p in g {
                    let Some(part_idx) = p.part_index else {
                        continue;
                    };
                    let opacity = opacities[part_idx];
                    for &link_part_idx in &p.link_part_indices {
                        param_ops::set_part_opacity(opacities, link_part_idx, opacity);
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct Pose3Json {
    #[serde(default, rename = "FadeInTime")]
    fade_in_time: Option<f32>,
    #[serde(rename = "Groups")]
    groups: Vec<Vec<PosePartJson>>,
}

#[derive(Debug, Deserialize)]
struct PosePartJson {
    #[serde(rename = "Id")]
    id: String,
    #[serde(default, rename = "Link")]
    link: Vec<String>,
}
