use crate::string::w2s_string;
use std::{cell::Cell, ffi::c_char};

thread_local! {
    static LAST_ERROR: Cell<Option<wasm2spirv::error::Error>> = Cell::new(None);
}

#[inline]
pub fn handle_error<T>(res: Result<T, wasm2spirv::error::Error>) -> Option<T> {
    match res {
        Ok(x) => return Some(x),
        Err(err) => LAST_ERROR.set(Some(err)),
    }
    return None;
}

pub unsafe extern "C" fn w2s_take_last_error_message() -> w2s_string {
    if let Some(error) = LAST_ERROR.take() {
        return w2s_string::new(error.to_string());
    } else {
        return core::mem::zeroed();
    }
}
