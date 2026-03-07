use crate::core::ffi;

#[derive(Clone, Copy, Debug, Default)]
pub struct CanvasInfo {
    pub size_in_pixels: [f32; 2],
    pub origin_in_pixels: [f32; 2],
    pub pixels_per_unit: f32,
}

pub struct Parameters<'a> {
    pub(crate) ids: &'a [*const std::ffi::c_char],
    pub(crate) values: &'a mut [f32],
    pub(crate) minimum_values: &'a [f32],
    pub(crate) maximum_values: &'a [f32],
    pub(crate) default_values: &'a [f32],
    pub(crate) types: &'a [ffi::csmParameterType],
    pub(crate) repeats: &'a [std::ffi::c_int],
}

impl<'a> Parameters<'a> {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn id(&self, index: usize) -> &std::ffi::CStr {
        unsafe { std::ffi::CStr::from_ptr(self.ids[index]) }
    }

    pub fn values_mut(&mut self) -> &mut [f32] {
        self.values
    }

    pub fn values(&self) -> &[f32] {
        self.values
    }

    pub fn default_values(&self) -> &[f32] {
        self.default_values
    }

    pub fn minimum_values(&self) -> &[f32] {
        self.minimum_values
    }

    pub fn maximum_values(&self) -> &[f32] {
        self.maximum_values
    }

    pub fn types(&self) -> &[ffi::csmParameterType] {
        self.types
    }

    pub fn repeats(&self) -> &[std::ffi::c_int] {
        self.repeats
    }

    pub fn is_repeat(&self, index: usize) -> bool {
        self.repeats[index] != 0
    }
}

pub struct Parts<'a> {
    pub(crate) ids: &'a [*const std::ffi::c_char],
    pub(crate) opacities: &'a mut [f32],
    pub(crate) parent_part_indices: &'a [i32],
}

impl<'a> Parts<'a> {
    pub fn len(&self) -> usize {
        self.opacities.len()
    }

    pub fn id(&self, index: usize) -> &std::ffi::CStr {
        unsafe { std::ffi::CStr::from_ptr(self.ids[index]) }
    }

    pub fn opacities_mut(&mut self) -> &mut [f32] {
        self.opacities
    }

    pub fn parent_part_indices(&self) -> &[i32] {
        self.parent_part_indices
    }
}

pub struct Drawables<'a> {
    pub(crate) ids: &'a [*const std::ffi::c_char],
    pub(crate) constant_flags: &'a [ffi::csmFlags],
    pub(crate) dynamic_flags: &'a [ffi::csmFlags],
    pub(crate) texture_indices: &'a [i32],
    pub(crate) draw_orders: &'a [i32],
    pub(crate) render_orders: &'a [i32],
    pub(crate) opacities: &'a [f32],
    pub(crate) mask_counts: &'a [i32],
    pub(crate) masks: &'a [*const i32],
    pub(crate) vertex_counts: &'a [i32],
    pub(crate) vertex_positions: &'a [*const ffi::csmVector2],
    pub(crate) vertex_uvs: &'a [*const ffi::csmVector2],
    pub(crate) index_counts: &'a [i32],
    pub(crate) indices: &'a [*const u16],
    pub(crate) multiply_colors: &'a [ffi::csmVector4],
    pub(crate) screen_colors: &'a [ffi::csmVector4],
    pub(crate) parent_part_indices: &'a [i32],
}

impl<'a> Drawables<'a> {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn id(&self, index: usize) -> &std::ffi::CStr {
        unsafe { std::ffi::CStr::from_ptr(self.ids[index]) }
    }

    pub fn constant_flags(&self) -> &[ffi::csmFlags] {
        self.constant_flags
    }

    pub fn dynamic_flags(&self) -> &[ffi::csmFlags] {
        self.dynamic_flags
    }

    pub fn texture_indices(&self) -> &[i32] {
        self.texture_indices
    }

    pub fn render_orders(&self) -> &[i32] {
        self.render_orders
    }

    pub fn draw_orders(&self) -> &[i32] {
        self.draw_orders
    }

    pub fn opacities(&self) -> &[f32] {
        self.opacities
    }

    pub fn mask_counts(&self) -> &[i32] {
        self.mask_counts
    }

    pub fn masks(&self, index: usize) -> Option<&'a [i32]> {
        let count = self.mask_counts[index];
        if count <= 0 {
            return None;
        }
        let count = count as usize;
        let ptr = self.masks[index];
        Some(unsafe { slice_from_raw_parts_allow_null(ptr, count) })
    }

    pub fn vertex_positions(&self, index: usize) -> &'a [ffi::csmVector2] {
        let count = self.vertex_counts[index] as usize;
        unsafe { slice_from_raw_parts_allow_null(self.vertex_positions[index], count) }
    }

    pub fn vertex_uvs(&self, index: usize) -> &'a [ffi::csmVector2] {
        let count = self.vertex_counts[index] as usize;
        unsafe { slice_from_raw_parts_allow_null(self.vertex_uvs[index], count) }
    }

    pub fn indices(&self, index: usize) -> &'a [u16] {
        let count = self.index_counts[index] as usize;
        unsafe { slice_from_raw_parts_allow_null(self.indices[index], count) }
    }

    pub fn multiply_color(&self, index: usize) -> ffi::csmVector4 {
        self.multiply_colors[index]
    }

    pub fn screen_color(&self, index: usize) -> ffi::csmVector4 {
        self.screen_colors[index]
    }

    pub fn parent_part_indices(&self) -> &[i32] {
        self.parent_part_indices
    }
}

pub(crate) fn usize_from_count(count: i32) -> Result<usize, Error> {
    if count < 0 {
        return Err(Error::InvalidCount(count));
    }
    Ok(count as usize)
}

pub(crate) unsafe fn slice_from_raw_parts_allow_null<'a, T>(
    ptr: *const T,
    len: usize,
) -> &'a [T] {
    use std::ptr::NonNull;
    if len == 0 {
        return &[];
    }
    let ptr = NonNull::new(ptr.cast_mut()).expect("Cubism Core returned null pointer");
    unsafe { std::slice::from_raw_parts(ptr.as_ptr(), len) }
}

pub(crate) unsafe fn slice_from_raw_parts_mut_allow_null<'a, T>(
    ptr: *mut T,
    len: usize,
) -> &'a mut [T] {
    use std::ptr::NonNull;
    if len == 0 {
        return &mut [];
    }
    let ptr = NonNull::new(ptr).expect("Cubism Core returned null pointer");
    unsafe { std::slice::from_raw_parts_mut(ptr.as_ptr(), len) }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("moc bytes are empty")]
    EmptyMoc,
    #[error("allocation failed (size={size}, align={align})")]
    AllocFailed { size: usize, align: usize },
    #[error("Core rejected moc")]
    InvalidMoc,
    #[error("Core failed to initialize model")]
    ModelInitFailed,
    #[error("Core returned invalid count: {0}")]
    InvalidCount(i32),
}
