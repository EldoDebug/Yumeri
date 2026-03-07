use std::{
    alloc::{Layout, alloc, dealloc},
    ptr::NonNull,
    sync::Arc,
};

use crate::core::ffi;
use crate::core::moc::Moc;
use crate::core::types::{
    CanvasInfo, Drawables, Error, Parameters, Parts, slice_from_raw_parts_allow_null,
    slice_from_raw_parts_mut_allow_null, usize_from_count,
};

pub struct Model {
    _moc: Arc<Moc>,
    mem: NonNull<u8>,
    layout: Layout,
    model: NonNull<ffi::csmModel>,
}

impl Model {
    pub fn new(moc: Arc<Moc>) -> Result<Self, Error> {
        let size = unsafe { ffi::csmGetSizeofModel(moc.as_ptr()) } as usize;
        if size == 0 {
            return Err(Error::ModelInitFailed);
        }

        let layout = Layout::from_size_align(size, ffi::csmAlignofModel as usize).unwrap();
        let ptr = unsafe { alloc(layout) };
        let Some(mem) = NonNull::new(ptr) else {
            return Err(Error::AllocFailed {
                size: layout.size(),
                align: layout.align(),
            });
        };

        let model_ptr = unsafe {
            ffi::csmInitializeModelInPlace(moc.as_ptr(), mem.as_ptr().cast::<_>(), size as u32)
        };
        let Some(model) = NonNull::new(model_ptr) else {
            unsafe { dealloc(mem.as_ptr(), layout) };
            return Err(Error::ModelInitFailed);
        };

        Ok(Self {
            _moc: moc,
            mem,
            layout,
            model,
        })
    }

    pub fn as_ptr(&self) -> *const ffi::csmModel {
        self.model.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut ffi::csmModel {
        self.model.as_ptr()
    }

    pub fn update(&mut self) {
        unsafe { ffi::csmUpdateModel(self.as_mut_ptr()) }
    }

    pub fn reset_drawable_dynamic_flags(&mut self) {
        unsafe { ffi::csmResetDrawableDynamicFlags(self.as_mut_ptr()) }
    }

    pub fn canvas_info(&self) -> CanvasInfo {
        let mut size = ffi::csmVector2::default();
        let mut origin = ffi::csmVector2::default();
        let mut ppu = 0.0f32;
        unsafe { ffi::csmReadCanvasInfo(self.as_ptr(), &mut size, &mut origin, &mut ppu) };
        CanvasInfo {
            size_in_pixels: [size.X, size.Y],
            origin_in_pixels: [origin.X, origin.Y],
            pixels_per_unit: ppu,
        }
    }

    pub fn parameters(&mut self) -> Result<Parameters<'_>, Error> {
        let count = unsafe { ffi::csmGetParameterCount(self.as_ptr()) };
        let count_usize = usize_from_count(count)?;
        let ids_ptr = unsafe { ffi::csmGetParameterIds(self.as_ptr()) };
        let values_ptr = unsafe { ffi::csmGetParameterValues(self.as_mut_ptr()) };
        let min_ptr = unsafe { ffi::csmGetParameterMinimumValues(self.as_ptr()) };
        let max_ptr = unsafe { ffi::csmGetParameterMaximumValues(self.as_ptr()) };
        let default_ptr = unsafe { ffi::csmGetParameterDefaultValues(self.as_ptr()) };
        let types_ptr = unsafe { ffi::csmGetParameterTypes(self.as_ptr()) };
        let repeats_ptr = unsafe { ffi::csmGetParameterRepeats(self.as_ptr()) };

        Ok(Parameters {
            ids: unsafe { slice_from_raw_parts_allow_null(ids_ptr, count_usize) },
            values: unsafe { slice_from_raw_parts_mut_allow_null(values_ptr, count_usize) },
            minimum_values: unsafe { slice_from_raw_parts_allow_null(min_ptr, count_usize) },
            maximum_values: unsafe { slice_from_raw_parts_allow_null(max_ptr, count_usize) },
            default_values: unsafe { slice_from_raw_parts_allow_null(default_ptr, count_usize) },
            types: unsafe { slice_from_raw_parts_allow_null(types_ptr, count_usize) },
            repeats: unsafe { slice_from_raw_parts_allow_null(repeats_ptr, count_usize) },
        })
    }

    pub fn parts(&mut self) -> Result<Parts<'_>, Error> {
        let count = unsafe { ffi::csmGetPartCount(self.as_ptr()) };
        let count_usize = usize_from_count(count)?;
        let ids_ptr = unsafe { ffi::csmGetPartIds(self.as_ptr()) };
        let opacities_ptr = unsafe { ffi::csmGetPartOpacities(self.as_mut_ptr()) };
        let parent_indices_ptr = unsafe { ffi::csmGetPartParentPartIndices(self.as_ptr()) };
        Ok(Parts {
            ids: unsafe { slice_from_raw_parts_allow_null(ids_ptr, count_usize) },
            opacities: unsafe { slice_from_raw_parts_mut_allow_null(opacities_ptr, count_usize) },
            parent_part_indices: unsafe {
                slice_from_raw_parts_allow_null(parent_indices_ptr, count_usize)
            },
        })
    }

    pub fn drawables(&mut self) -> Result<Drawables<'_>, Error> {
        let count = unsafe { ffi::csmGetDrawableCount(self.as_ptr()) };
        let count_usize = usize_from_count(count)?;

        let ids = unsafe {
            slice_from_raw_parts_allow_null(ffi::csmGetDrawableIds(self.as_ptr()), count_usize)
        };
        let constant_flags = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableConstantFlags(self.as_ptr()),
                count_usize,
            )
        };
        let dynamic_flags = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableDynamicFlags(self.as_ptr()),
                count_usize,
            )
        };
        let texture_indices = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableTextureIndices(self.as_ptr()),
                count_usize,
            )
        };
        let draw_orders = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableDrawOrders(self.as_ptr()),
                count_usize,
            )
        };
        let render_orders = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableRenderOrders(self.as_ptr()),
                count_usize,
            )
        };
        let opacities = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableOpacities(self.as_ptr()),
                count_usize,
            )
        };
        let mask_counts = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableMaskCounts(self.as_ptr()),
                count_usize,
            )
        };
        let masks = unsafe {
            slice_from_raw_parts_allow_null(ffi::csmGetDrawableMasks(self.as_ptr()), count_usize)
        };
        let vertex_counts = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableVertexCounts(self.as_ptr()),
                count_usize,
            )
        };
        let vertex_positions = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableVertexPositions(self.as_ptr()),
                count_usize,
            )
        };
        let vertex_uvs = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableVertexUvs(self.as_ptr()),
                count_usize,
            )
        };
        let index_counts = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableIndexCounts(self.as_ptr()),
                count_usize,
            )
        };
        let indices = unsafe {
            slice_from_raw_parts_allow_null(ffi::csmGetDrawableIndices(self.as_ptr()), count_usize)
        };
        let multiply_colors = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableMultiplyColors(self.as_ptr()),
                count_usize,
            )
        };
        let screen_colors = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableScreenColors(self.as_ptr()),
                count_usize,
            )
        };
        let parent_part_indices = unsafe {
            slice_from_raw_parts_allow_null(
                ffi::csmGetDrawableParentPartIndices(self.as_ptr()),
                count_usize,
            )
        };

        Ok(Drawables {
            ids,
            constant_flags,
            dynamic_flags,
            texture_indices,
            draw_orders,
            render_orders,
            opacities,
            mask_counts,
            masks,
            vertex_counts,
            vertex_positions,
            vertex_uvs,
            index_counts,
            indices,
            multiply_colors,
            screen_colors,
            parent_part_indices,
        })
    }
}

impl Drop for Model {
    fn drop(&mut self) {
        unsafe { dealloc(self.mem.as_ptr(), self.layout) }
    }
}

// SAFETY: Model owns its memory exclusively and the Cubism Core library
// documents that separate Model instances can be used from different threads.
unsafe impl Send for Model {}
