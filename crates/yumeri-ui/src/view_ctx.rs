use std::any::TypeId;

use yumeri_animation::animator::Animator;

use crate::callback::AnyCallback;
use crate::event::EventPayload;
use crate::template_provider::TemplateProvider;
use crate::template_view_builder::TemplateViewBuilder;

pub struct ViewCtx {
    owner_type: TypeId,
    animator: *mut Animator,
    template_provider: Option<*const dyn TemplateProvider>,
}

impl ViewCtx {
    pub(crate) fn new(owner_type: TypeId, animator: &mut Animator) -> Self {
        Self {
            owner_type,
            animator,
            template_provider: None,
        }
    }

    pub(crate) fn with_template_provider_ptr(
        mut self,
        provider: Option<*const dyn TemplateProvider>,
    ) -> Self {
        self.template_provider = provider;
        self
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

    pub fn template(&self, name: &str) -> TemplateViewBuilder {
        TemplateViewBuilder::new(name, self.template_provider)
    }
}
