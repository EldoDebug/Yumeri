use crate::framework::{
    DisplayInfoError, ExpressionError, ModelSettingError, MotionError, PhysicsError, PoseError,
    UserDataError,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Loader(#[from] crate::loader::LoaderError),
    #[error(transparent)]
    ModelSetting(#[from] ModelSettingError),
    #[error(transparent)]
    Motion(#[from] MotionError),
    #[error(transparent)]
    Expression(#[from] ExpressionError),
    #[error(transparent)]
    DisplayInfo(#[from] DisplayInfoError),
    #[error(transparent)]
    Pose(#[from] PoseError),
    #[error(transparent)]
    Physics(#[from] PhysicsError),
    #[error(transparent)]
    UserData(#[from] UserDataError),
    #[error(transparent)]
    Core(#[from] crate::core::Error),
    #[error("missing required parameter: {0}")]
    MissingParameter(&'static str),
    #[error("no motion group: {0}")]
    NoMotionGroup(String),
}
