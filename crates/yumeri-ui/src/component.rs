use std::any::{Any, TypeId};

use crate::element::Element;
use crate::event_ctx::EventCtx;
use crate::view_ctx::ViewCtx;

pub trait Component: 'static {
    fn view(&self, ctx: &mut ViewCtx) -> Element;
    fn on_mount(&mut self, _ctx: &mut EventCtx) {}
    fn on_unmount(&mut self, _ctx: &mut EventCtx) {}
}

// Type-erased component stored in UiTree nodes
pub(crate) struct ComponentBox {
    inner: Option<Box<dyn Any>>,
    type_id: TypeId,
    view_fn: fn(&dyn Any, &mut ViewCtx) -> Element,
    on_mount_fn: fn(&mut dyn Any, &mut EventCtx),
    on_unmount_fn: fn(&mut dyn Any, &mut EventCtx),
}

impl ComponentBox {
    pub fn new<C: Component>(component: C) -> Self {
        Self {
            inner: Some(Box::new(component)),
            type_id: TypeId::of::<C>(),
            view_fn: |any, ctx| {
                any.downcast_ref::<C>()
                    .expect("ComponentBox type mismatch in view")
                    .view(ctx)
            },
            on_mount_fn: |any, ctx| {
                any.downcast_mut::<C>()
                    .expect("ComponentBox type mismatch in on_mount")
                    .on_mount(ctx);
            },
            on_unmount_fn: |any, ctx| {
                any.downcast_mut::<C>()
                    .expect("ComponentBox type mismatch in on_unmount")
                    .on_unmount(ctx);
            },
        }
    }

    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    pub fn view(&self, ctx: &mut ViewCtx) -> Element {
        let inner = self
            .inner
            .as_ref()
            .expect("ComponentBox::view called while inner is taken");
        (self.view_fn)(inner.as_ref(), ctx)
    }

    pub fn on_mount(&mut self, ctx: &mut EventCtx) {
        let inner = self
            .inner
            .as_mut()
            .expect("ComponentBox::on_mount called while inner is taken");
        (self.on_mount_fn)(inner.as_mut(), ctx);
    }

    pub fn on_unmount(&mut self, ctx: &mut EventCtx) {
        let inner = self
            .inner
            .as_mut()
            .expect("ComponentBox::on_unmount called while inner is taken");
        (self.on_unmount_fn)(inner.as_mut(), ctx);
    }

    pub fn take(&mut self) -> Option<Box<dyn Any>> {
        self.inner.take()
    }

    pub fn put_back(&mut self, inner: Box<dyn Any>) {
        self.inner = Some(inner);
    }

    #[allow(dead_code)]
    pub fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        self.inner.as_mut().map(|b| b.as_mut())
    }
}
