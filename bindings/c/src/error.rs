use crate::string::w2s_string;
use std::cell::Cell;

thread_local! {
    static LAST_ERROR: Cell<Option<wasm2spirv::error::Error>> = Cell::new(None);
}

#[inline]
pub fn handle_error<T, E: Into<wasm2spirv::error::Error>>(res: Result<T, E>) -> Option<T> {
    match res {
        Ok(x) => return Some(x),
        Err(err) => LAST_ERROR.with(|cell| cell.set(Some(err.into()))),
    }
    return None;
}

#[no_mangle]
pub unsafe extern "C" fn w2s_take_last_error_message() -> w2s_string {
    if let Some(error) = LAST_ERROR.with(Cell::take) {
        return w2s_string::new(error.to_string());
    } else {
        return core::mem::zeroed();
    }
}
