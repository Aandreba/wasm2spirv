use crate::string::{w2s_string, w2s_string_view};
use std::{cell::Cell, os::raw::c_void, panic::PanicInfo};

thread_local! {
    static LAST_ERROR: Cell<Option<wasm2spirv::error::Error>> = Cell::new(None);
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct w2s_panic_info {
    payload: w2s_string_view,
    location: w2s_panic_location,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct w2s_panic_location {
    file: w2s_string_view,
    line: u32,
    column: u32,
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

/// # Safety
/// Both `f` and `user_data` must be thread safe
#[no_mangle]
pub unsafe extern "C" fn w2s_set_panic_handler(
    f: unsafe extern "C" fn(w2s_panic_info, *mut c_void),
    user_data: *mut c_void,
) {
    #[repr(transparent)]
    struct SafePtr(*mut c_void);
    unsafe impl Send for SafePtr {}
    unsafe impl Sync for SafePtr {}

    let user_data = SafePtr(user_data);
    std::panic::set_hook(Box::new(move |info| unsafe {
        let info_payload = info.payload();

        let payload;
        if let Some(pl) = info_payload.downcast_ref::<String>() {
            payload = w2s_string_view::new_str(pl);
        } else if let Some(pl) = info_payload.downcast_ref::<&'static str>() {
            payload = w2s_string_view::new_str(pl);
        } else {
            payload = core::mem::zeroed();
        }

        let location = match info.location() {
            Some(location) => w2s_panic_location {
                file: w2s_string_view::new_str(location.file()),
                line: location.line(),
                column: location.column(),
            },
            None => core::mem::zeroed(),
        };

        let user_data = &user_data;
        f(w2s_panic_info { payload, location }, user_data.0);
    }))
}

/// Useful in WebAssembly contexts.
#[no_mangle]
pub unsafe extern "C" fn w2s_set_imported_panic_handler(user_data: *mut c_void) {
    #[link(wasm_import_module = "w2s_panic_handler")]
    extern "C" {
        fn w2s_panic_handler(info: w2s_panic_info, user_data: *mut c_void);
    }

    return w2s_set_panic_handler(w2s_panic_handler, user_data);
}
