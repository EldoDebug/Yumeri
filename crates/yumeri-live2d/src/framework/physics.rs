use glam::Vec2;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, thiserror::Error)]
pub enum PhysicsError {
    #[error("failed to parse physics3 json")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Copy)]
pub struct PhysicsOptions {
    pub gravity: Vec2,
    pub wind: Vec2,
}

impl Default for PhysicsOptions {
    /// Default forces matching the Cubism SDK convention.
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, -1.0),
            wind: Vec2::ZERO,
        }
    }
}

#[derive(Debug)]
pub struct Physics {
    fps: f32,
    sub_rigs: Vec<SubRig>,

    options: PhysicsOptions,
    current_remain_time: f32,
    parameter_caches: Vec<f32>,
    parameter_input_caches: Vec<f32>,
    previous_rig_outputs: Vec<Vec<f32>>,
    current_rig_outputs: Vec<Vec<f32>>,
}

#[derive(Debug)]
struct SubRig {
    normalization_position: Normalization,
    normalization_angle: Normalization,
    inputs: Vec<PhysicsInput>,
    outputs: Vec<PhysicsOutput>,
    particles: Vec<PhysicsParticle>,
}

#[derive(Debug, Clone, Copy)]
struct Normalization {
    min: f32,
    max: f32,
    default: f32,
}

#[derive(Debug, Clone, Copy)]
enum PhysicsSource {
    X,
    Y,
    Angle,
}

#[derive(Debug)]
struct PhysicsInput {
    source_id: String,
    source_parameter_index: Option<usize>,
    weight: f32,
    input_type: PhysicsSource,
    reflect: bool,
}

#[derive(Debug)]
struct PhysicsOutput {
    destination_id: String,
    destination_parameter_index: Option<usize>,
    vertex_index: usize,
    scale: f32,
    weight: f32,
    output_type: PhysicsSource,
    reflect: bool,
    value_below_minimum: f32,
    value_exceeded_maximum: f32,
}

#[derive(Debug, Clone, Copy)]
struct PhysicsParticle {
    mobility: f32,
    delay: f32,
    acceleration: f32,
    radius: f32,
    position: Vec2,
    last_position: Vec2,
    last_gravity: Vec2,
    force: Vec2,
    velocity: Vec2,
    initial_position: Vec2,
}

impl Physics {
    /// Parse a physics (`.physics3.json`) document and initialize simulation state.
    pub fn parse(json_text: &str) -> Result<Self, PhysicsError> {
        let json: Physics3Json = serde_json::from_str(json_text)?;

        let mut sub_rigs = Vec::new();
        for setting in json.physics_settings {
            let normalization_position = Normalization {
                min: setting.normalization.position.minimum,
                max: setting.normalization.position.maximum,
                default: setting.normalization.position.default,
            };
            let normalization_angle = Normalization {
                min: setting.normalization.angle.minimum,
                max: setting.normalization.angle.maximum,
                default: setting.normalization.angle.default,
            };

            let inputs = setting
                .input
                .into_iter()
                .map(|i| PhysicsInput {
                    source_id: i.source.id,
                    source_parameter_index: None,
                    weight: i.weight,
                    input_type: match i.input_type.as_str() {
                        "Y" => PhysicsSource::Y,
                        "Angle" => PhysicsSource::Angle,
                        _ => PhysicsSource::X,
                    },
                    reflect: i.reflect,
                })
                .collect::<Vec<_>>();

            let outputs = setting
                .output
                .into_iter()
                .map(|o| PhysicsOutput {
                    destination_id: o.destination.id,
                    destination_parameter_index: None,
                    vertex_index: o.vertex_index.max(0) as usize,
                    scale: o.scale,
                    weight: o.weight,
                    output_type: match o.output_type.as_str() {
                        "Y" => PhysicsSource::Y,
                        "Angle" => PhysicsSource::Angle,
                        _ => PhysicsSource::X,
                    },
                    reflect: o.reflect,
                    value_below_minimum: f32::INFINITY,
                    value_exceeded_maximum: f32::NEG_INFINITY,
                })
                .collect::<Vec<_>>();

            let particles = setting
                .vertices
                .into_iter()
                .map(|v| PhysicsParticle {
                    mobility: v.mobility,
                    delay: v.delay,
                    acceleration: v.acceleration,
                    radius: v.radius,
                    position: Vec2::ZERO,
                    last_position: Vec2::ZERO,
                    last_gravity: Vec2::new(0.0, 1.0),
                    force: Vec2::ZERO,
                    velocity: Vec2::ZERO,
                    initial_position: Vec2::ZERO,
                })
                .collect::<Vec<_>>();

            sub_rigs.push(SubRig {
                normalization_position,
                normalization_angle,
                inputs,
                outputs,
                particles,
            });
        }

        let previous_rig_outputs = sub_rigs
            .iter()
            .map(|s| vec![0.0; s.outputs.len()])
            .collect::<Vec<_>>();
        let current_rig_outputs = sub_rigs
            .iter()
            .map(|s| vec![0.0; s.outputs.len()])
            .collect::<Vec<_>>();

        let mut out = Self {
            fps: json.meta.fps,
            sub_rigs,
            options: PhysicsOptions {
                gravity: Vec2::new(
                    json.meta.effective_forces.gravity.x,
                    json.meta.effective_forces.gravity.y,
                ),
                wind: Vec2::new(
                    json.meta.effective_forces.wind.x,
                    json.meta.effective_forces.wind.y,
                ),
            },
            current_remain_time: 0.0,
            parameter_caches: Vec::new(),
            parameter_input_caches: Vec::new(),
            previous_rig_outputs,
            current_rig_outputs,
        };

        out.initialize();
        Ok(out)
    }

    /// Override effective forces used by the simulation.
    pub fn set_options(&mut self, options: PhysicsOptions) {
        self.options = options;
    }

    /// Return the currently configured effective forces.
    pub fn options(&self) -> PhysicsOptions {
        self.options
    }

    /// Reset simulation state to defaults.
    pub fn reset(&mut self) {
        self.options = PhysicsOptions::default();
        self.current_remain_time = 0.0;
        self.initialize();
    }

    /// Initialize particle state for all sub-rigs.
    fn initialize(&mut self) {
        for rig in &mut self.sub_rigs {
            if rig.particles.is_empty() {
                continue;
            }

            rig.particles[0].initial_position = Vec2::ZERO;
            rig.particles[0].last_position = rig.particles[0].initial_position;
            rig.particles[0].last_gravity = Vec2::new(0.0, 1.0);
            rig.particles[0].velocity = Vec2::ZERO;
            rig.particles[0].force = Vec2::ZERO;

            for i in 1..rig.particles.len() {
                let radius = Vec2::new(0.0, rig.particles[i].radius);
                let init = rig.particles[i - 1].initial_position + radius;
                rig.particles[i].initial_position = init;
                rig.particles[i].position = init;
                rig.particles[i].last_position = init;
                rig.particles[i].last_gravity = Vec2::new(0.0, 1.0);
                rig.particles[i].velocity = Vec2::ZERO;
                rig.particles[i].force = Vec2::ZERO;
            }
        }
    }

    /// Advance the simulation and write the resulting outputs back into model parameters.
    pub fn evaluate(
        &mut self,
        model: &mut crate::core::Model,
        delta_time_seconds: f32,
        param_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        if delta_time_seconds <= 0.0 {
            return Ok(());
        }

        const AIR_RESISTANCE: f32 = 5.0;
        const MAX_DELTA_TIME: f32 = 5.0;
        const MOVEMENT_THRESHOLD: f32 = 0.001;
        const MAXIMUM_WEIGHT: f32 = 100.0;

        self.current_remain_time += delta_time_seconds;
        if self.current_remain_time > MAX_DELTA_TIME {
            self.current_remain_time = 0.0;
        }

        let param_count = { model.parameters()?.len() };
        if self.parameter_caches.len() < param_count {
            self.parameter_caches.resize(param_count, 0.0);
        }
        if self.parameter_input_caches.len() < param_count {
            let params = model.parameters()?;
            self.parameter_input_caches = params.values().to_vec();
        }

        let physics_delta_time = if self.fps > 0.0 {
            1.0 / self.fps
        } else {
            delta_time_seconds
        };

        while self.current_remain_time >= physics_delta_time {
            for (prev, cur) in self
                .previous_rig_outputs
                .iter_mut()
                .zip(self.current_rig_outputs.iter())
            {
                prev.copy_from_slice(cur);
            }

            let input_weight = physics_delta_time / self.current_remain_time;
            {
                let params = model.parameters()?;
                for i in 0..param_count {
                    let v = self.parameter_input_caches[i] * (1.0 - input_weight)
                        + params.values()[i] * input_weight;
                    self.parameter_caches[i] = v;
                    self.parameter_input_caches[i] = v;
                }
            }

            for (setting_index, rig) in self.sub_rigs.iter_mut().enumerate() {
                let mut total_angle = 0.0f32;
                let mut total_translation = Vec2::ZERO;

                for input in &mut rig.inputs {
                    let w = input.weight / MAXIMUM_WEIGHT;
                    if w <= 0.0 {
                        continue;
                    }

                    if input.source_parameter_index.is_none() {
                        input.source_parameter_index = param_index.get(&input.source_id).copied();
                    }
                    let Some(src_idx) = input.source_parameter_index else {
                        continue;
                    };

                    let (min, max, def) = {
                        let params = model.parameters()?;
                        (
                            params.minimum_values()[src_idx],
                            params.maximum_values()[src_idx],
                            params.default_values()[src_idx],
                        )
                    };
                    let v = self.parameter_caches[src_idx];
                    match input.input_type {
                        PhysicsSource::X => {
                            total_translation.x += normalize_parameter_value(
                                v,
                                min,
                                max,
                                def,
                                rig.normalization_position.min,
                                rig.normalization_position.max,
                                rig.normalization_position.default,
                                input.reflect,
                            ) * w;
                        }
                        PhysicsSource::Y => {
                            total_translation.y += normalize_parameter_value(
                                v,
                                min,
                                max,
                                def,
                                rig.normalization_position.min,
                                rig.normalization_position.max,
                                rig.normalization_position.default,
                                input.reflect,
                            ) * w;
                        }
                        PhysicsSource::Angle => {
                            total_angle += normalize_parameter_value(
                                v,
                                min,
                                max,
                                def,
                                rig.normalization_angle.min,
                                rig.normalization_angle.max,
                                rig.normalization_angle.default,
                                input.reflect,
                            ) * w;
                        }
                    }
                }

                let rad_angle = degrees_to_radian(-total_angle);
                let cos = rad_angle.cos();
                let sin = rad_angle.sin();
                let x = total_translation.x * cos - total_translation.y * sin;
                total_translation.y = x * sin + total_translation.y * cos;
                total_translation.x = x;

                let threshold_value = MOVEMENT_THRESHOLD * rig.normalization_position.max;
                update_particles(
                    &mut rig.particles,
                    total_translation,
                    total_angle,
                    self.options.wind,
                    threshold_value,
                    physics_delta_time,
                    AIR_RESISTANCE,
                );

                for (out_i, output) in rig.outputs.iter_mut().enumerate() {
                    if output.destination_parameter_index.is_none() {
                        output.destination_parameter_index =
                            param_index.get(&output.destination_id).copied();
                    }
                    let Some(dst_idx) = output.destination_parameter_index else {
                        continue;
                    };
                    if output.vertex_index < 1 || output.vertex_index >= rig.particles.len() {
                        continue;
                    }

                    let translation = rig.particles[output.vertex_index].position
                        - rig.particles[output.vertex_index - 1].position;
                    let mut out_value = match output.output_type {
                        PhysicsSource::X => translation.x,
                        PhysicsSource::Y => translation.y,
                        PhysicsSource::Angle => {
                            let parent_gravity = if output.vertex_index >= 2 {
                                rig.particles[output.vertex_index - 1].position
                                    - rig.particles[output.vertex_index - 2].position
                            } else {
                                -self.options.gravity
                            };
                            direction_to_radian(parent_gravity, translation)
                        }
                    };

                    if output.reflect {
                        out_value *= -1.0;
                    }

                    self.current_rig_outputs[setting_index][out_i] = out_value;

                    let (min, max) = {
                        let params = model.parameters()?;
                        (
                            params.minimum_values()[dst_idx],
                            params.maximum_values()[dst_idx],
                        )
                    };
                    let is_repeat = { model.parameters()?.is_repeat(dst_idx) };

                    update_output_parameter_value(
                        &mut self.parameter_caches[dst_idx],
                        min,
                        max,
                        is_repeat,
                        out_value,
                        output,
                    );
                }
            }

            self.current_remain_time -= physics_delta_time;
        }

        let alpha = if physics_delta_time > 0.0 {
            self.current_remain_time / physics_delta_time
        } else {
            0.0
        };

        self.interpolate(model, alpha, param_index)
    }

    /// Blend between the previous and current rig outputs and apply them to the model.
    fn interpolate(
        &mut self,
        model: &mut crate::core::Model,
        weight: f32,
        param_index: &HashMap<String, usize>,
    ) -> Result<(), crate::core::Error> {
        let weight = weight.clamp(0.0, 1.0);
        let mut params = model.parameters()?;

        for (setting_index, rig) in self.sub_rigs.iter_mut().enumerate() {
            for (i, output) in rig.outputs.iter_mut().enumerate() {
                if output.destination_parameter_index.is_none() {
                    output.destination_parameter_index =
                        param_index.get(&output.destination_id).copied();
                }
                let Some(dst_idx) = output.destination_parameter_index else {
                    continue;
                };
                let v = self.previous_rig_outputs[setting_index][i] * (1.0 - weight)
                    + self.current_rig_outputs[setting_index][i] * weight;
                let min = params.minimum_values()[dst_idx];
                let max = params.maximum_values()[dst_idx];
                let is_repeat = params.is_repeat(dst_idx);
                update_output_parameter_value(
                    &mut params.values_mut()[dst_idx],
                    min,
                    max,
                    is_repeat,
                    v,
                    output,
                );
            }
        }

        Ok(())
    }
}

/// Convert degrees to radians.
fn degrees_to_radian(degrees: f32) -> f32 {
    (degrees / 180.0) * core::f32::consts::PI
}

/// Return the signed angle between `from` and `to` in radians.
fn direction_to_radian(from: Vec2, to: Vec2) -> f32 {
    let q1 = to.y.atan2(to.x);
    let q2 = from.y.atan2(from.x);
    let mut ret = q1 - q2;
    while ret < -core::f32::consts::PI {
        ret += core::f32::consts::PI * 2.0;
    }
    while ret > core::f32::consts::PI {
        ret -= core::f32::consts::PI * 2.0;
    }
    ret
}

/// Build a direction vector from `radian`.
fn radian_to_direction(radian: f32) -> Vec2 {
    Vec2::new(radian.sin(), radian.cos())
}

/// Normalize an input parameter value into the simulation's normalized domain.
fn normalize_parameter_value(
    mut value: f32,
    parameter_minimum: f32,
    parameter_maximum: f32,
    _parameter_default: f32,
    normalized_minimum: f32,
    normalized_maximum: f32,
    normalized_default: f32,
    is_inverted: bool,
) -> f32 {
    let max_value = parameter_maximum.max(parameter_minimum);
    if max_value < value {
        value = max_value;
    }
    let min_value = parameter_maximum.min(parameter_minimum);
    if min_value > value {
        value = min_value;
    }

    let min_norm_value = normalized_minimum.min(normalized_maximum);
    let max_norm_value = normalized_minimum.max(normalized_maximum);
    let middle_norm_value = normalized_default;

    let middle_value = min_value + (max_value - min_value).abs() / 2.0;
    let param_value = value - middle_value;

    let result = match sign(param_value) {
        1 => {
            let n_length = max_norm_value - middle_norm_value;
            let p_length = max_value - middle_value;
            if p_length.abs() > f32::EPSILON {
                param_value * (n_length / p_length) + middle_norm_value
            } else {
                middle_norm_value
            }
        }
        -1 => {
            let n_length = min_norm_value - middle_norm_value;
            let p_length = min_value - middle_value;
            if p_length.abs() > f32::EPSILON {
                param_value * (n_length / p_length) + middle_norm_value
            } else {
                middle_norm_value
            }
        }
        _ => middle_norm_value,
    };

    if is_inverted { result } else { result * -1.0 }
}

/// Return the sign of `value` as -1, 0, or 1.
fn sign(value: f32) -> i32 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

/// Update particle positions/velocities for one simulation step.
fn update_particles(
    particles: &mut [PhysicsParticle],
    total_translation: Vec2,
    total_angle: f32,
    wind_direction: Vec2,
    threshold_value: f32,
    delta_time_seconds: f32,
    air_resistance: f32,
) {
    if particles.is_empty() {
        return;
    }

    particles[0].position = total_translation;

    let total_radian = degrees_to_radian(total_angle);
    let mut current_gravity = radian_to_direction(total_radian);
    current_gravity = normalize_vec2(current_gravity);

    for i in 1..particles.len() {
        particles[i].force = current_gravity * particles[i].acceleration + wind_direction;
        particles[i].last_position = particles[i].position;

        let delay = particles[i].delay * delta_time_seconds * 30.0;

        let mut direction = particles[i].position - particles[i - 1].position;
        let radian =
            direction_to_radian(particles[i].last_gravity, current_gravity) / air_resistance;
        let cos = radian.cos();
        let sin = radian.sin();
        let x = cos * direction.x - direction.y * sin;
        direction.y = sin * x + direction.y * cos;
        direction.x = x;

        particles[i].position = particles[i - 1].position + direction;

        let velocity = particles[i].velocity * delay;
        let force = particles[i].force * delay * delay;
        particles[i].position = particles[i].position + velocity + force;

        let mut new_direction = particles[i].position - particles[i - 1].position;
        new_direction = normalize_vec2(new_direction);
        particles[i].position = particles[i - 1].position + new_direction * particles[i].radius;

        if particles[i].position.x.abs() < threshold_value {
            particles[i].position.x = 0.0;
        }

        if delay.abs() > f32::EPSILON {
            let v = (particles[i].position - particles[i].last_position) / delay;
            particles[i].velocity = v * particles[i].mobility;
        }

        particles[i].force = Vec2::ZERO;
        particles[i].last_gravity = current_gravity;
    }
}

/// Normalize a vector, returning `Vec2::ZERO` for near-zero inputs.
fn normalize_vec2(v: Vec2) -> Vec2 {
    let len = v.length();
    if len.abs() <= f32::EPSILON {
        Vec2::ZERO
    } else {
        v / len
    }
}

/// Apply an output value to a destination parameter, handling clamping and repeat behavior.
fn update_output_parameter_value(
    parameter_value: &mut f32,
    parameter_min: f32,
    parameter_max: f32,
    is_repeat: bool,
    translation: f32,
    output: &mut PhysicsOutput,
) {
    const MAXIMUM_WEIGHT: f32 = 100.0;

    let output_scale = output.scale;
    let mut value = translation * output_scale;

    if is_repeat {
        value = repeat_value(parameter_min, parameter_max, value);
    } else if value < parameter_min {
        if value < output.value_below_minimum {
            output.value_below_minimum = value;
        }
        value = parameter_min;
    } else if value > parameter_max {
        if value > output.value_exceeded_maximum {
            output.value_exceeded_maximum = value;
        }
        value = parameter_max;
    }

    let w = (output.weight / MAXIMUM_WEIGHT).clamp(0.0, 1.0);
    if w >= 1.0 {
        *parameter_value = value;
    } else {
        *parameter_value = *parameter_value * (1.0 - w) + value * w;
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

#[derive(Debug, Deserialize)]
struct Physics3Json {
    #[serde(rename = "Meta")]
    meta: PhysicsMeta,
    #[serde(rename = "PhysicsSettings")]
    physics_settings: Vec<PhysicsSettingJson>,
}

#[derive(Debug, Deserialize)]
struct PhysicsMeta {
    #[serde(rename = "Fps")]
    fps: f32,
    #[serde(rename = "EffectiveForces")]
    effective_forces: EffectiveForcesJson,
}

#[derive(Debug, Deserialize)]
struct EffectiveForcesJson {
    #[serde(rename = "Gravity")]
    gravity: Vec2Json,
    #[serde(rename = "Wind")]
    wind: Vec2Json,
}

#[derive(Debug, Deserialize)]
struct Vec2Json {
    #[serde(rename = "X")]
    x: f32,
    #[serde(rename = "Y")]
    y: f32,
}

#[derive(Debug, Deserialize)]
struct PhysicsSettingJson {
    #[serde(rename = "Input")]
    input: Vec<InputJson>,
    #[serde(rename = "Output")]
    output: Vec<OutputJson>,
    #[serde(rename = "Vertices")]
    vertices: Vec<VertexJson>,
    #[serde(rename = "Normalization")]
    normalization: NormalizationJson,
}

#[derive(Debug, Deserialize)]
struct NormalizationJson {
    #[serde(rename = "Position")]
    position: NormalizationAxisJson,
    #[serde(rename = "Angle")]
    angle: NormalizationAxisJson,
}

#[derive(Debug, Deserialize)]
struct NormalizationAxisJson {
    #[serde(rename = "Minimum")]
    minimum: f32,
    #[serde(rename = "Maximum")]
    maximum: f32,
    #[serde(rename = "Default")]
    default: f32,
}

#[derive(Debug, Deserialize)]
struct InputJson {
    #[serde(rename = "Source")]
    source: ParamRefJson,
    #[serde(rename = "Weight")]
    weight: f32,
    #[serde(rename = "Type")]
    input_type: String,
    #[serde(rename = "Reflect")]
    reflect: bool,
}

#[derive(Debug, Deserialize)]
struct OutputJson {
    #[serde(rename = "Destination")]
    destination: ParamRefJson,
    #[serde(rename = "VertexIndex")]
    vertex_index: i32,
    #[serde(rename = "Scale")]
    scale: f32,
    #[serde(rename = "Weight")]
    weight: f32,
    #[serde(rename = "Type")]
    output_type: String,
    #[serde(rename = "Reflect")]
    reflect: bool,
}

#[derive(Debug, Deserialize)]
struct ParamRefJson {
    #[serde(rename = "Target")]
    _target: String,
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct VertexJson {
    #[serde(rename = "Mobility")]
    mobility: f32,
    #[serde(rename = "Delay")]
    delay: f32,
    #[serde(rename = "Acceleration")]
    acceleration: f32,
    #[serde(rename = "Radius")]
    radius: f32,
}
