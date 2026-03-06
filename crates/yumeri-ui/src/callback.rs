use std::any::{Any, TypeId};

use crate::event::EventPayload;

pub struct AnyCallback {
    target_type: TypeId,
    invoke: Box<dyn FnMut(&mut dyn Any, &EventPayload)>,
}

impl AnyCallback {
    pub fn new<C: 'static>(mut f: impl FnMut(&mut C, &EventPayload) + 'static) -> Self {
        Self {
            target_type: TypeId::of::<C>(),
            invoke: Box::new(move |any, payload| {
                let component = any.downcast_mut::<C>().unwrap_or_else(|| {
                    panic!(
                        "AnyCallback type mismatch: expected {:?}",
                        TypeId::of::<C>()
                    )
                });
                f(component, payload);
            }),
        }
    }

    pub fn invoke(&mut self, target: &mut dyn Any, payload: &EventPayload) {
        (self.invoke)(target, payload);
    }

    pub fn target_type(&self) -> TypeId {
        self.target_type
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_invokes_correctly() {
        struct Counter {
            count: i32,
        }

        let mut cb = AnyCallback::new(|c: &mut Counter, _| {
            c.count += 1;
        });

        let mut counter = Counter { count: 0 };
        cb.invoke(&mut counter, &EventPayload::Click);
        assert_eq!(counter.count, 1);

        cb.invoke(&mut counter, &EventPayload::Click);
        assert_eq!(counter.count, 2);
    }

    #[test]
    #[should_panic(expected = "AnyCallback type mismatch")]
    fn callback_panics_on_type_mismatch() {
        struct A;
        struct B;

        let mut cb = AnyCallback::new(|_: &mut A, _| {});
        let mut b = B;
        cb.invoke(&mut b, &EventPayload::Click);
    }
}
