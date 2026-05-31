//! Deliberately malformed plugin: it exports a valid ABI string and the
//! interner hook but omits `maxima_plugin_register`. The loader must reject it
//! with a clear "missing symbol" error rather than crash. Used by
//! crates/eval/tests/plugin_ux_test.rs.

use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn maxima_plugin_abi() -> *const c_char {
    maxima_plugin::abi_cstr().as_ptr()
}

#[no_mangle]
pub extern "C" fn maxima_plugin_set_interner(
    ptr: *mut std::sync::Mutex<maxima_plugin::maxima_core::InternTable>,
) {
    unsafe { maxima_plugin::maxima_core::adopt_interner(ptr) };
}

// Intentionally no `maxima_plugin_register`.
