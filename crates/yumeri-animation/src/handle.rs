use std::marker::PhantomData;

use crate::interpolate::Interpolate;

/// Unique identifier for an animation within an `Animator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AnimationId(pub(crate) u64);

/// Unique identifier for a timeline within an `Animator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineId(pub(crate) u64);

/// Type-safe handle to a running animation of type `T`.
pub struct Handle<T: Interpolate> {
    pub(crate) id: AnimationId,
    _marker: PhantomData<T>,
}

impl<T: Interpolate> Handle<T> {
    pub(crate) fn new(id: AnimationId) -> Self {
        Self {
            id,
            _marker: PhantomData,
        }
    }

    pub fn raw(&self) -> RawHandle {
        RawHandle { id: self.id }
    }
}

impl<T: Interpolate> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: Interpolate> Copy for Handle<T> {}

/// Type-erased handle used by `Timeline`, `pause()`, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawHandle {
    pub(crate) id: AnimationId,
}

impl<T: Interpolate> From<Handle<T>> for RawHandle {
    fn from(h: Handle<T>) -> Self {
        h.raw()
    }
}

/// Handle to a running timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimelineHandle {
    pub(crate) id: TimelineId,
}
