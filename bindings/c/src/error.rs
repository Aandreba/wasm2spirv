use std::{cell::Cell, ffi::c_char};

thread_local! {
    static LAST_ERROR: Cell<Option<wasm2spirv::error::Error>> = Cell::new(None);
}

pub unsafe extern "C" fn w2s_take_last_error_message() -> *const c_char {
    if let Some(error) = LAST_ERROR.take() {
    } else {
        return core::ptr::null();
    }
}
