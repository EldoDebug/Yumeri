use std::any::TypeId;

use yumeri_animation::animator::Animator;

use crate::callback::AnyCallback;
use crate::event::EventPayload;

pub struct ViewCtx {
    owner_type: TypeId,
    animator: *mut Animator,
}

impl ViewCtx {
    pub(crate) fn new(owner_type: TypeId, animator: &mut Animator) -> Self {
        Self {
            owner_type,
            animator,
        }
    }

    pub fn callback<C: 'static>(
        &mut self,
        f: impl FnMut(&mut C, &EventPayload) + 'static,
    ) -> AnyCallback {
        assert_eq!(
            self.owner_type,
            TypeId::of::<C>(),
            "ViewCtx::callback type must match the owning component"
        );
        AnyCallback::new(f)
    }

    pub fn animator(&mut self) -> &mut Animator {
        // SAFETY: The pointer is valid for the duration of view() calls.
        // UiTree ensures this by holding a mutable reference to the Animator
        // throughout the view phase.
        unsafe { &mut *self.animator }
    }
}
