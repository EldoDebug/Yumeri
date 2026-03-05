pub mod animator;
pub mod easing;
pub mod handle;
pub mod interpolate;
pub mod keyframes;
pub mod playback;
pub mod stagger;
pub mod timeline;
pub mod tween;

pub mod prelude {
    pub use crate::animator::{AnimationEvent, Animator};
    pub use crate::easing::Easing;
    pub use crate::handle::{Handle, RawHandle, TimelineHandle};
    pub use crate::interpolate::Interpolate;
    pub use crate::keyframes::Keyframes;
    pub use crate::playback::{Direction, LoopMode, PlaybackState};
    pub use crate::stagger::{stagger, StaggerConfig, StaggerFrom};
    pub use crate::timeline::{TimeOffset, Timeline};
    pub use crate::tween::Tween;
}
