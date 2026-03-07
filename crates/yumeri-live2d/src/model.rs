use crate::core::{CanvasInfo, Drawables, Moc, Model};
use crate::error::Error;
use crate::framework::{
    param_ops, Breath, DisplayInfo, Expression, ExpressionManager, EyeBlink, LipSync, ModelMatrix,
    ModelSetting, MotionClip, MotionPriority, MotionQueueManager, Physics, Pose, TargetPoint,
    UserData, VirtualParameters,
};
use crate::loader::AssetLoader;
use rand::prelude::IndexedRandom as _;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct Live2DModel {
    setting: ModelSetting,
    textures: Vec<PathBuf>,
    canvas: CanvasInfo,
    model: Model,
    model_matrix: ModelMatrix,
    model_opacity: f32,
    display_info: Option<DisplayInfo>,

    param_index: HashMap<String, usize>,
    part_index: HashMap<String, usize>,
    drawable_index: HashMap<String, usize>,
    saved_params: Vec<f32>,
    saved_model_opacity: f32,
    virtual_params: VirtualParameters,

    drag: TargetPoint,
    time: f32,

    motion_groups: HashMap<String, Vec<Arc<MotionClip>>>,
    expressions: Vec<(String, Arc<Expression>)>,

    motion_queue: MotionQueueManager,
    expression_manager: ExpressionManager,
    motion_events: Vec<String>,

    eye_blink: EyeBlink,
    eye_blink_params: Vec<(String, usize)>,
    eye_blink_ids: Vec<String>,

    lip_sync: LipSync,
    lip_sync_param_indices: Vec<usize>,
    lip_sync_ids: Vec<String>,

    breath: Breath,
    pose: Option<Pose>,
    physics: Option<Physics>,
    user_data: Option<UserData>,

    idx_angle_x: Option<usize>,
    idx_angle_y: Option<usize>,
    idx_angle_z: Option<usize>,
    idx_body_angle_x: Option<usize>,
    idx_eye_ball_x: Option<usize>,
    idx_eye_ball_y: Option<usize>,
}

impl Live2DModel {
    pub fn load<L: AssetLoader>(loader: &L, model3_path: &Path) -> Result<Self, Error> {
        let model_dir = model3_path
            .parent()
            .ok_or_else(|| {
                crate::loader::LoaderError::ReadFailed {
                    path: model3_path.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidInput,
                        "model3 has no parent dir",
                    ),
                }
            })?
            .to_path_buf();

        let json_text = loader.load_string(model3_path)?;
        let setting = ModelSetting::parse(model_dir, &json_text)?;

        let moc_bytes = loader.load_bytes(&setting.moc_path())?;
        let moc = Moc::from_bytes(&moc_bytes)?;
        let mut model = Model::new(moc)?;

        let canvas = model.canvas_info();

        let mut param_index = HashMap::new();
        let mut saved_params = Vec::new();
        {
            let params = model.parameters()?;
            saved_params.extend_from_slice(params.values());
            for i in 0..params.len() {
                param_index.insert(params.id(i).to_string_lossy().to_string(), i);
            }
        }

        let mut part_index = HashMap::new();
        {
            let parts = model.parts()?;
            for i in 0..parts.len() {
                part_index.insert(parts.id(i).to_string_lossy().to_string(), i);
            }
        }

        let mut drawable_index = HashMap::new();
        {
            let drawables = model.drawables()?;
            for i in 0..drawables.len() {
                drawable_index.insert(drawables.id(i).to_string_lossy().to_string(), i);
            }
        }

        let textures = setting.texture_paths().collect::<Vec<_>>();

        let motion_groups = load_motions(loader, &setting)?;
        let expressions = load_expressions(loader, &setting)?;

        let pose = if let Some(p) = setting.pose_path() {
            let text = loader.load_string(&p)?;
            Some(Pose::parse(&text)?)
        } else {
            None
        };

        let physics = if let Some(p) = setting.physics_path() {
            let text = loader.load_string(&p)?;
            Some(Physics::parse(&text)?)
        } else {
            None
        };

        let user_data = if let Some(p) = setting.user_data_path() {
            let text = loader.load_string(&p)?;
            Some(UserData::parse(&text)?)
        } else {
            None
        };

        let display_info = if let Some(p) = setting.display_info_path() {
            let text = loader.load_string(&p)?;
            Some(DisplayInfo::parse(&text)?)
        } else {
            None
        };

        let idx_angle_x = param_index.get("ParamAngleX").copied();
        let idx_angle_y = param_index.get("ParamAngleY").copied();
        let idx_angle_z = param_index.get("ParamAngleZ").copied();
        let idx_body_angle_x = param_index.get("ParamBodyAngleX").copied();
        let idx_eye_ball_x = param_index.get("ParamEyeBallX").copied();
        let idx_eye_ball_y = param_index.get("ParamEyeBallY").copied();

        let eye_blink_ids = setting.group_ids("EyeBlink").to_vec();
        let eye_blink_params = setting
            .group_ids("EyeBlink")
            .iter()
            .filter_map(|id| param_index.get(id).copied().map(|idx| (id.clone(), idx)))
            .collect::<Vec<_>>();

        let lip_sync_ids = setting.group_ids("LipSync").to_vec();
        let lip_sync_param_indices = setting
            .group_ids("LipSync")
            .iter()
            .filter_map(|id| param_index.get(id).copied())
            .collect::<Vec<_>>();

        let ppu = if canvas.pixels_per_unit > 0.0 {
            canvas.pixels_per_unit
        } else {
            1.0
        };
        let base_w = (canvas.size_in_pixels[0] / ppu).max(0.001);
        let base_h = (canvas.size_in_pixels[1] / ppu).max(0.001);
        let mut model_matrix = ModelMatrix::new(base_w, base_h);
        if let Some(layout) = setting.layout() {
            model_matrix.setup_from_layout(layout);
        }

        Ok(Self {
            setting,
            textures,
            canvas,
            model,
            model_matrix,
            model_opacity: 1.0,
            display_info,
            param_index,
            part_index,
            drawable_index,
            saved_params,
            saved_model_opacity: 1.0,
            virtual_params: VirtualParameters::default(),
            drag: TargetPoint::default(),
            time: 0.0,
            motion_groups,
            expressions,
            motion_queue: MotionQueueManager::default(),
            expression_manager: ExpressionManager::default(),
            motion_events: Vec::new(),
            eye_blink: EyeBlink::default(),
            eye_blink_params,
            eye_blink_ids,
            lip_sync: LipSync::default(),
            lip_sync_param_indices,
            lip_sync_ids,
            breath: Breath::default(),
            pose,
            physics,
            user_data,
            idx_angle_x,
            idx_angle_y,
            idx_angle_z,
            idx_body_angle_x,
            idx_eye_ball_x,
            idx_eye_ball_y,
        })
    }

    pub fn texture_paths(&self) -> &[PathBuf] {
        &self.textures
    }

    pub fn drawables(&mut self) -> Result<Drawables<'_>, crate::core::Error> {
        self.model.drawables()
    }

    pub fn model_matrix(&self) -> &ModelMatrix {
        &self.model_matrix
    }

    pub fn model_opacity(&self) -> f32 {
        self.model_opacity
    }

    pub fn canvas_info(&self) -> &CanvasInfo {
        &self.canvas
    }

    pub fn core_model(&self) -> &Model {
        &self.model
    }

    pub fn core_model_mut(&mut self) -> &mut Model {
        &mut self.model
    }

    pub fn setting(&self) -> &ModelSetting {
        &self.setting
    }

    pub fn display_info(&self) -> Option<&DisplayInfo> {
        self.display_info.as_ref()
    }

    pub fn set_dragging(&mut self, x: f32, y: f32) {
        self.drag.set(x, y);
    }

    pub fn set_lip_sync_value(&mut self, value_0_to_1: f32) {
        self.lip_sync.set_value(value_0_to_1);
    }

    pub fn take_motion_events(&mut self) -> Vec<String> {
        std::mem::take(&mut self.motion_events)
    }

    pub fn start_random_motion(
        &mut self,
        group: &str,
        priority: MotionPriority,
    ) -> Result<(), Error> {
        if !self.motion_queue.reserve_motion(priority) {
            return Ok(());
        }

        let Some(list) = self.motion_groups.get(group) else {
            return Err(Error::NoMotionGroup(group.to_string()));
        };
        let mut rng = rand::rng();
        let Some(clip) = list.choose(&mut rng) else {
            return Err(Error::NoMotionGroup(group.to_string()));
        };
        self.motion_queue.start_motion(Arc::clone(clip), priority);
        Ok(())
    }

    pub fn set_random_expression(&mut self) -> Result<(), Error> {
        let mut rng = rand::rng();
        let Some((_, expr)) = self.expressions.choose(&mut rng) else {
            return Ok(());
        };
        self.expression_manager.start_expression(Arc::clone(expr));
        Ok(())
    }

    pub fn update(&mut self, dt: f32) -> Result<(), Error> {
        self.time += dt;
        self.drag.update(dt);

        if self.motion_queue.is_empty() {
            let _ = self.start_random_motion("Idle", MotionPriority::Idle);
        }

        self.model_opacity = self.saved_model_opacity;

        {
            let mut params = self.model.parameters()?;
            if params.values().len() != self.saved_params.len() {
                self.saved_params = params.values().to_vec();
            } else {
                params.values_mut().copy_from_slice(&self.saved_params);
            }
        }

        if !self.motion_queue.is_empty() {
            self.motion_queue.update(
                &mut self.model,
                dt,
                &mut self.virtual_params,
                &mut self.model_opacity,
                &self.param_index,
                &self.part_index,
                &self.eye_blink_ids,
                &self.lip_sync_ids,
                &mut self.motion_events,
            )?;
        }

        {
            let params = self.model.parameters()?;
            self.saved_params.clear();
            self.saved_params.extend_from_slice(params.values());
        }
        self.saved_model_opacity = self.model_opacity;

        self.expression_manager
            .update(&mut self.model, dt, &self.param_index)?;

        let motion_affects_eye_blink = self
            .eye_blink_params
            .iter()
            .any(|(id, _)| self.motion_queue.affects_parameter_id(id))
            || self.motion_queue.affects_model_curve("EyeBlink");
        let expr_affects_eye_blink = self
            .eye_blink_params
            .iter()
            .any(|(id, _)| self.expression_manager.affects_parameter_id(id));

        if !self.eye_blink_params.is_empty()
            && !motion_affects_eye_blink
            && !expr_affects_eye_blink
        {
            let open = self.eye_blink.update(dt);
            let mut params = self.model.parameters()?;
            for (_, idx) in &self.eye_blink_params {
                param_ops::set_parameter_value(&mut params, *idx, open, 1.0);
            }
        }

        let motion_affects_lip_sync = self
            .lip_sync_ids
            .iter()
            .any(|id| self.motion_queue.affects_parameter_id(id))
            || self.motion_queue.affects_model_curve("LipSync");
        let expr_affects_lip_sync = self
            .lip_sync_ids
            .iter()
            .any(|id| self.expression_manager.affects_parameter_id(id));

        if !self.lip_sync_param_indices.is_empty()
            && !motion_affects_lip_sync
            && !expr_affects_lip_sync
        {
            self.lip_sync
                .apply(&mut self.model, &self.lip_sync_param_indices, 1.0)?;
        }

        let drag_x = self.drag.x();
        let drag_y = self.drag.y();

        let mut params = self.model.parameters()?;
        if let Some(idx) = self.idx_angle_x {
            param_ops::add_parameter_value(&mut params, idx, drag_x * 30.0, 1.0);
        }
        if let Some(idx) = self.idx_angle_y {
            param_ops::add_parameter_value(&mut params, idx, drag_y * 30.0, 1.0);
        }
        if let Some(idx) = self.idx_angle_z {
            param_ops::add_parameter_value(
                &mut params,
                idx,
                drag_x * drag_y * -30.0,
                1.0,
            );
        }
        if let Some(idx) = self.idx_body_angle_x {
            param_ops::add_parameter_value(&mut params, idx, drag_x * 10.0, 1.0);
        }
        if let Some(idx) = self.idx_eye_ball_x {
            param_ops::add_parameter_value(&mut params, idx, drag_x, 1.0);
        }
        if let Some(idx) = self.idx_eye_ball_y {
            param_ops::add_parameter_value(&mut params, idx, drag_y, 1.0);
        }
        drop(params);

        self.breath.update(&mut self.model, dt, &self.param_index)?;

        if let Some(physics) = &mut self.physics {
            physics.evaluate(&mut self.model, dt, &self.param_index)?;
        }

        if let Some(pose) = &mut self.pose {
            pose.update_parameters(
                &mut self.model,
                &mut self.virtual_params,
                dt,
                &self.param_index,
                &self.part_index,
            )?;
        }

        self.model.update();

        Ok(())
    }

    pub fn hit_test_model_space(&mut self, hit_area_name: &str, x: f32, y: f32) -> bool {
        let Some(hit_id) = self.setting.hit_area_id(hit_area_name) else {
            return false;
        };
        let Some(&drawable_index) = self.drawable_index.get(hit_id) else {
            return false;
        };

        let Ok(drawables) = self.model.drawables() else {
            return false;
        };

        let verts = drawables.vertex_positions(drawable_index);
        if verts.is_empty() {
            return false;
        }

        let (min_x, min_y, max_x, max_y) = aabb(verts);
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    pub fn hit_test_view_space(&mut self, hit_area_name: &str, x: f32, y: f32) -> bool {
        let mx = self.model_matrix.invert_transform_x(x);
        let my = self.model_matrix.invert_transform_y(y);
        self.hit_test_model_space(hit_area_name, mx, my)
    }

    pub fn hit_test_art_mesh_user_data_model_space(&mut self, x: f32, y: f32) -> Vec<String> {
        let Some(user_data) = &self.user_data else {
            return Vec::new();
        };
        let Ok(drawables) = self.model.drawables() else {
            return Vec::new();
        };

        let mut out = Vec::new();
        for n in user_data.art_mesh_nodes() {
            let Some(&drawable_index) = self.drawable_index.get(&n.id) else {
                continue;
            };
            let verts = drawables.vertex_positions(drawable_index);
            if verts.is_empty() {
                continue;
            }

            let (min_x, min_y, max_x, max_y) = aabb(verts);
            if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                out.push(n.value.clone());
            }
        }
        out
    }

    pub fn hit_test_art_mesh_user_data_view_space(&mut self, x: f32, y: f32) -> Vec<String> {
        let mx = self.model_matrix.invert_transform_x(x);
        let my = self.model_matrix.invert_transform_y(y);
        self.hit_test_art_mesh_user_data_model_space(mx, my)
    }

    pub fn reset_drawable_dynamic_flags(&mut self) {
        self.model.reset_drawable_dynamic_flags();
    }
}

fn aabb(verts: &[crate::core::ffi::csmVector2]) -> (f32, f32, f32, f32) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for v in verts {
        min_x = min_x.min(v.X);
        min_y = min_y.min(v.Y);
        max_x = max_x.max(v.X);
        max_y = max_y.max(v.Y);
    }
    (min_x, min_y, max_x, max_y)
}

fn load_motions<L: AssetLoader>(
    loader: &L,
    setting: &ModelSetting,
) -> Result<HashMap<String, Vec<Arc<MotionClip>>>, Error> {
    let mut out: HashMap<String, Vec<Arc<MotionClip>>> = HashMap::new();
    for (group, refs) in setting.motions_with_fades() {
        let mut clips = Vec::new();
        for r in refs {
            let text = loader.load_string(&r.path)?;
            let mut clip = MotionClip::parse(&text)?;

            if let Some(t) = r.fade_in_time.filter(|t| *t >= 0.0) {
                clip.fade_in_time = t;
            }
            if let Some(t) = r.fade_out_time.filter(|t| *t >= 0.0) {
                clip.fade_out_time = t;
            }
            clips.push(Arc::new(clip));
        }
        out.insert(group.to_string(), clips);
    }
    Ok(out)
}

fn load_expressions<L: AssetLoader>(
    loader: &L,
    setting: &ModelSetting,
) -> Result<Vec<(String, Arc<Expression>)>, Error> {
    let mut out = Vec::new();
    for (name, path) in setting.expressions() {
        let text = loader.load_string(&path)?;
        let expr = Expression::parse(&text)?;
        out.push((name.to_string(), Arc::new(expr)));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::StdFsLoader;

    #[test]
    #[ignore]
    fn mao_model_updates_with_phase2_features() -> Result<(), Error> {
        let loader = StdFsLoader;
        let model3_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/mao-wgpu/assets/Mao/Mao.model3.json");
        let mut model = Live2DModel::load(&loader, &model3_path)?;

        model.set_lip_sync_value(0.5);

        for _ in 0..120 {
            model.update(1.0 / 30.0)?;
            let _ = model.take_motion_events();
        }

        Ok(())
    }
}
