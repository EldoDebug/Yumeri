pub mod ffi;
pub mod moc;
pub mod model;
pub mod types;

use std::ffi::CStr;

pub use self::ffi as sys;
pub use self::moc::Moc;
pub use self::model::Model;
pub use self::types::{CanvasInfo, Drawables, Error, Parameters, Parts};

pub fn install_tracing_logger() {
    unsafe { ffi::csmSetLogFunction(Some(cubism_core_log_callback)) }
}

pub fn clear_log_function() {
    unsafe { ffi::csmSetLogFunction(None) }
}

unsafe extern "C" fn cubism_core_log_callback(message: *const core::ffi::c_char) {
    let _ = std::panic::catch_unwind(|| {
        if message.is_null() {
            tracing::info!(target: "live2d_cubism_core", "(null)");
            return;
        }
        let msg = unsafe { CStr::from_ptr(message) }.to_string_lossy();
        tracing::info!(target: "live2d_cubism_core", "{}", msg);
    });
}
