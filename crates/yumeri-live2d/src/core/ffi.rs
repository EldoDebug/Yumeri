#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct csmMoc {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct csmModel {
    _unused: [u8; 0],
}
#[doc = " Cubism version identifier."]
pub type csmVersion = core::ffi::c_uint;
#[doc = " Necessary alignment for mocs (in bytes)."]
pub const csmAlignofMoc: _bindgen_ty_1 = 64;
#[doc = " Necessary alignment for models (in bytes)."]
pub const csmAlignofModel: _bindgen_ty_1 = 16;
#[doc = " Alignment constraints."]
pub type _bindgen_ty_1 = core::ffi::c_int;
#[doc = " Additive blend mode mask."]
pub const csmBlendAdditive: _bindgen_ty_2 = 1;
#[doc = " Multiplicative blend mode mask."]
pub const csmBlendMultiplicative: _bindgen_ty_2 = 2;
#[doc = " Double-sidedness mask."]
pub const csmIsDoubleSided: _bindgen_ty_2 = 4;
#[doc = " Clipping mask inversion mode mask."]
pub const csmIsInvertedMask: _bindgen_ty_2 = 8;
#[doc = " Bit masks for non-dynamic drawable flags."]
pub type _bindgen_ty_2 = core::ffi::c_int;
#[doc = " Flag set when visible."]
pub const csmIsVisible: _bindgen_ty_3 = 1;
#[doc = " Flag set when visibility did change."]
pub const csmVisibilityDidChange: _bindgen_ty_3 = 2;
#[doc = " Flag set when opacity did change."]
pub const csmOpacityDidChange: _bindgen_ty_3 = 4;
#[doc = " Flag set when draw order did change."]
pub const csmDrawOrderDidChange: _bindgen_ty_3 = 8;
#[doc = " Flag set when render order did change."]
pub const csmRenderOrderDidChange: _bindgen_ty_3 = 16;
#[doc = " Flag set when vertex positions did change."]
pub const csmVertexPositionsDidChange: _bindgen_ty_3 = 32;
#[doc = " Flag set when blend color did change."]
pub const csmBlendColorDidChange: _bindgen_ty_3 = 64;
#[doc = " Bit masks for dynamic drawable flags."]
pub type _bindgen_ty_3 = core::ffi::c_int;
#[doc = " Bitfield."]
pub type csmFlags = core::ffi::c_uchar;
#[doc = " unknown"]
pub const csmMocVersion_Unknown: _bindgen_ty_4 = 0;
#[doc = " moc3 file version 3.0.00 - 3.2.07"]
pub const csmMocVersion_30: _bindgen_ty_4 = 1;
#[doc = " moc3 file version 3.3.00 - 3.3.03"]
pub const csmMocVersion_33: _bindgen_ty_4 = 2;
#[doc = " moc3 file version 4.0.00 - 4.1.05"]
pub const csmMocVersion_40: _bindgen_ty_4 = 3;
#[doc = " moc3 file version 4.2.00 - 4.2.04"]
pub const csmMocVersion_42: _bindgen_ty_4 = 4;
#[doc = " moc3 file version 5.0.00 -"]
pub const csmMocVersion_50: _bindgen_ty_4 = 5;
#[doc = " moc3 file format version."]
pub type _bindgen_ty_4 = core::ffi::c_int;
#[doc = " moc3 version identifier."]
pub type csmMocVersion = core::ffi::c_uint;
#[doc = " Normal parameter."]
pub const csmParameterType_Normal: _bindgen_ty_5 = 0;
#[doc = " Parameter for blend shape."]
pub const csmParameterType_BlendShape: _bindgen_ty_5 = 1;
#[doc = " Parameter types."]
pub type _bindgen_ty_5 = core::ffi::c_int;
#[doc = " Parameter type."]
pub type csmParameterType = core::ffi::c_int;
#[doc = " 2 component vector."]
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct csmVector2 {
    #[doc = " First component."]
    pub X: f32,
    #[doc = " Second component."]
    pub Y: f32,
}
#[doc = " 4 component vector."]
#[repr(C)]
#[derive(Debug, Default, Copy, Clone)]
pub struct csmVector4 {
    #[doc = " 1st component."]
    pub X: f32,
    #[doc = " 2nd component."]
    pub Y: f32,
    #[doc = " 3rd component."]
    pub Z: f32,
    #[doc = " 4th component."]
    pub W: f32,
}
#[doc = " Log handler.\n\n @param  message  Null-terminated string message to log."]
pub type csmLogFunction =
    ::core::option::Option<unsafe extern "C" fn(message: *const core::ffi::c_char)>;
macro_rules! extern_block {
    ($abi:literal) => {
        unsafe extern $abi {
            #[doc = " Queries Core version.\n\n @return  Core version."]
            pub fn csmGetVersion() -> csmVersion;
            #[doc = " Gets Moc file supported latest version.\n\n @return csmMocVersion (Moc file latest format version)."]
            pub fn csmGetLatestMocVersion() -> csmMocVersion;
            #[doc = " Gets Moc file format version.\n\n @param  address  Address of moc.\n @param  size     Size of moc (in bytes).\n\n @return csmMocVersion"]
            pub fn csmGetMocVersion(
                address: *const core::ffi::c_void,
                size: core::ffi::c_uint,
            ) -> csmMocVersion;
            #[doc = " Checks consistency of a moc.\n\n @param  address  Address of unrevived moc. The address must be aligned to 'csmAlignofMoc'.\n @param  size     Size of moc (in bytes).\n\n @return  '1' if Moc is valid; '0' otherwise."]
            pub fn csmHasMocConsistency(
                address: *mut core::ffi::c_void,
                size: core::ffi::c_uint,
            ) -> core::ffi::c_int;
            #[doc = " Queries log handler.\n\n @return  Log handler."]
            pub fn csmGetLogFunction() -> csmLogFunction;
            #[doc = " Sets log handler.\n\n @param  handler  Handler to use."]
            pub fn csmSetLogFunction(handler: csmLogFunction);
            #[doc = " Tries to revive a moc from bytes in place.\n\n @param  address  Address of unrevived moc. The address must be aligned to 'csmAlignofMoc'.\n @param  size     Size of moc (in bytes).\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmReviveMocInPlace(
                address: *mut core::ffi::c_void,
                size: core::ffi::c_uint,
            ) -> *mut csmMoc;
            #[doc = " Queries size of a model in bytes.\n\n @param  moc  Moc to query.\n\n @return  Valid size on success; '0' otherwise."]
            pub fn csmGetSizeofModel(moc: *const csmMoc) -> core::ffi::c_uint;
            #[doc = " Tries to instantiate a model in place.\n\n @param  moc      Source moc.\n @param  address  Address to place instance at. Address must be aligned to 'csmAlignofModel'.\n @param  size     Size of memory block for instance (in bytes).\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmInitializeModelInPlace(
                moc: *const csmMoc,
                address: *mut core::ffi::c_void,
                size: core::ffi::c_uint,
            ) -> *mut csmModel;
            #[doc = " Updates a model.\n\n @param  model  Model to update."]
            pub fn csmUpdateModel(model: *mut csmModel);
            #[doc = " Reads info on a model canvas.\n\n @param  model              Model query.\n\n @param  outSizeInPixels    Canvas dimensions.\n @param  outOriginInPixels  Origin of model on canvas.\n @param  outPixelsPerUnit   Aspect used for scaling pixels to units."]
            pub fn csmReadCanvasInfo(
                model: *const csmModel,
                outSizeInPixels: *mut csmVector2,
                outOriginInPixels: *mut csmVector2,
                outPixelsPerUnit: *mut f32,
            );
            #[doc = " Gets number of parameters.\n\n @param[in]  model  Model to query.\n\n @return  Valid count on success; '-1' otherwise."]
            pub fn csmGetParameterCount(model: *const csmModel) -> core::ffi::c_int;
            #[doc = " Gets parameter IDs.\n All IDs are null-terminated ANSI strings.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterIds(model: *const csmModel) -> *mut *const core::ffi::c_char;
            #[doc = " Gets parameter types.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterTypes(model: *const csmModel) -> *const csmParameterType;
            #[doc = " Gets minimum parameter values.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterMinimumValues(model: *const csmModel) -> *const f32;
            #[doc = " Gets maximum parameter values.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterMaximumValues(model: *const csmModel) -> *const f32;
            #[doc = " Gets default parameter values.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterDefaultValues(model: *const csmModel) -> *const f32;
            #[doc = " Gets read/write parameter values buffer.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterValues(model: *mut csmModel) -> *mut f32;
            #[doc = " Gets Parameter Repeat informations.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterRepeats(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets number of key values of each parameter.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterKeyCounts(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets key values of each parameter.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetParameterKeyValues(model: *const csmModel) -> *mut *const f32;
            #[doc = " Gets number of parts.\n\n @param  model  Model to query.\n\n @return  Valid count on success; '-1' otherwise."]
            pub fn csmGetPartCount(model: *const csmModel) -> core::ffi::c_int;
            #[doc = " Gets parts IDs.\n All IDs are null-terminated ANSI strings.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetPartIds(model: *const csmModel) -> *mut *const core::ffi::c_char;
            #[doc = " Gets read/write part opacities buffer.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetPartOpacities(model: *mut csmModel) -> *mut f32;
            #[doc = " Gets part's parent part indices.\n\n @param   model   Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetPartParentPartIndices(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets number of drawables.\n\n @param  model  Model to query.\n\n @return  Valid count on success; '-1' otherwise."]
            pub fn csmGetDrawableCount(model: *const csmModel) -> core::ffi::c_int;
            #[doc = " Gets drawable IDs.\n All IDs are null-terminated ANSI strings.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableIds(model: *const csmModel) -> *mut *const core::ffi::c_char;
            #[doc = " Gets constant drawable flags.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableConstantFlags(model: *const csmModel) -> *const csmFlags;
            #[doc = " Gets dynamic drawable flags.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableDynamicFlags(model: *const csmModel) -> *const csmFlags;
            #[doc = " Gets drawable texture indices.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableTextureIndices(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets drawable draw orders.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableDrawOrders(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets drawable render orders.\n The higher the order, the more up front a drawable is.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0'otherwise."]
            pub fn csmGetDrawableRenderOrders(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets drawable opacities.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableOpacities(model: *const csmModel) -> *const f32;
            #[doc = " Gets numbers of masks of each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableMaskCounts(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets mask indices of each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableMasks(model: *const csmModel) -> *mut *const core::ffi::c_int;
            #[doc = " Gets number of vertices of each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableVertexCounts(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets vertex position data of each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; a null pointer otherwise."]
            pub fn csmGetDrawableVertexPositions(model: *const csmModel) -> *mut *const csmVector2;
            #[doc = " Gets texture coordinate data of each drawables.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableVertexUvs(model: *const csmModel) -> *mut *const csmVector2;
            #[doc = " Gets number of triangle indices for each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableIndexCounts(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Gets triangle index data for each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableIndices(model: *const csmModel) -> *mut *const core::ffi::c_ushort;
            #[doc = " Gets multiply color data for each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableMultiplyColors(model: *const csmModel) -> *const csmVector4;
            #[doc = " Gets screen color data for each drawable.\n\n @param  model  Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableScreenColors(model: *const csmModel) -> *const csmVector4;
            #[doc = " Gets drawable's parent part indices.\n\n @param   model   Model to query.\n\n @return  Valid pointer on success; '0' otherwise."]
            pub fn csmGetDrawableParentPartIndices(model: *const csmModel) -> *const core::ffi::c_int;
            #[doc = " Resets all dynamic drawable flags.\n\n @param  model  Model containing flags."]
            pub fn csmResetDrawableDynamicFlags(model: *mut csmModel);
        }
    };
}

#[cfg(all(windows, target_arch = "x86", feature = "dynamic"))]
extern_block!("system");

#[cfg(not(all(windows, target_arch = "x86", feature = "dynamic")))]
extern_block!("C");
