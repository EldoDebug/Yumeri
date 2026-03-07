use std::{
    alloc::{Layout, alloc, dealloc},
    ptr::NonNull,
    sync::Arc,
};

use crate::core::ffi;
use crate::core::types::Error;

pub struct Moc {
    mem: NonNull<u8>,
    layout: Layout,
    size: usize,
    moc: NonNull<ffi::csmMoc>,
}

impl Moc {
    pub fn from_bytes(moc3: &[u8]) -> Result<Arc<Self>, Error> {
        if moc3.is_empty() {
            return Err(Error::EmptyMoc);
        }

        let layout = Layout::from_size_align(moc3.len(), ffi::csmAlignofMoc as usize).unwrap();
        let ptr = unsafe { alloc(layout) };
        let Some(mem) = NonNull::new(ptr) else {
            return Err(Error::AllocFailed {
                size: layout.size(),
                align: layout.align(),
            });
        };

        unsafe {
            std::ptr::copy_nonoverlapping(moc3.as_ptr(), mem.as_ptr(), moc3.len());
        }

        let moc_ptr =
            unsafe { ffi::csmReviveMocInPlace(mem.as_ptr().cast::<_>(), moc3.len() as u32) };
        let Some(moc) = NonNull::new(moc_ptr) else {
            unsafe { dealloc(mem.as_ptr(), layout) };
            return Err(Error::InvalidMoc);
        };

        Ok(Arc::new(Self {
            mem,
            layout,
            size: moc3.len(),
            moc,
        }))
    }

    pub fn as_ptr(&self) -> *const ffi::csmMoc {
        self.moc.as_ptr()
    }

    pub fn moc_version(&self) -> ffi::csmMocVersion {
        unsafe { ffi::csmGetMocVersion(self.mem.as_ptr().cast::<_>(), self.size as u32) }
    }
}

impl Drop for Moc {
    fn drop(&mut self) {
        unsafe { dealloc(self.mem.as_ptr(), self.layout) }
    }
}

// SAFETY: Moc data is immutable after construction and the Cubism Core library
// documents that separate Moc instances can be used from different threads.
unsafe impl Send for Moc {}
unsafe impl Sync for Moc {}
