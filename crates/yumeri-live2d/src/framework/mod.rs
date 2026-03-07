pub mod breath;
pub mod display_info;
pub mod expression;
pub mod eye_blink;
pub mod lip_sync;
pub mod model_matrix;
pub mod model_setting;
pub mod motion;
pub mod motion_queue;
pub mod param_ops;
pub mod physics;
pub mod pose;
pub mod target_point;
pub mod user_data;

use std::collections::HashMap;

pub type VirtualParameters = HashMap<String, f32>;

pub use breath::{Breath, BreathParameter};
pub use display_info::{DisplayInfo, DisplayInfoError, DisplayParameter, DisplayParameterGroup, DisplayPart};
pub use expression::{Expression, ExpressionBlend, ExpressionError, ExpressionManager, ExpressionParameter};
pub use eye_blink::EyeBlink;
pub use lip_sync::LipSync;
pub use model_matrix::ModelMatrix;
pub use model_setting::{
    ExpressionRef, FileReferences, Group, HitArea, Model3Json, ModelSetting, ModelSettingError,
    MotionRef, ResolvedMotionRef,
};
pub use motion::{Curve, CurveTarget, MotionClip, MotionError, MotionEvent};
pub use motion_queue::{MotionPriority, MotionQueueManager};
pub use param_ops::{
    add_parameter_value, clamp01, easing_sine01, multiply_parameter_value, set_parameter_value,
    set_part_opacity,
};
pub use physics::{Physics, PhysicsError, PhysicsOptions};
pub use pose::{Pose, PoseError};
pub use target_point::TargetPoint;
pub use user_data::{UserData, UserDataError, UserDataNode};
