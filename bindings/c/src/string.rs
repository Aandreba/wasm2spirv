use std::{
    ffi::{c_char, CString},
    mem::ManuallyDrop,
};

pub type w2s_string_view = w2s_view<u8>;
pub type w2s_byte_view = w2s_view<u8>;
pub type w2s_word_view = w2s_view<u32>;

/// A view into a UTF-8 string
#[repr(C)]
pub struct w2s_view<T> {
    ptr: *const T,
    len: usize,
}

impl<T> w2s_view<T> {
    #[inline]
    pub unsafe fn new(v: &[T]) -> Self {
        return Self {
            ptr: v.as_ptr(),
            len: v.len(),
        };
    }

    pub unsafe fn as_slice(&self) -> &[T] {
        core::slice::from_raw_parts(self.ptr, self.len)
    }
}

impl w2s_string_view {
    #[inline]
    pub unsafe fn new_str(s: &str) -> Self {
        return Self::new(s.as_bytes());
    }

    pub unsafe fn as_str(&self) -> &str {
        core::str::from_utf8_unchecked(self.as_slice())
    }
}

/// A UTF-8, null terminated, string
#[repr(C)]
pub struct w2s_string {
    ptr: *const c_char,
    len: usize,
}

impl w2s_string {
    pub unsafe fn new(str: String) -> Self {
        let len = str.len();
        let ptr =
            CString::into_raw(CString::new(str).expect("unexpected null-terminator mid-string"));
        return Self { ptr, len };
    }

    #[inline]
    pub unsafe fn as_str(&self) -> Option<&str> {
        if self.ptr.is_null() {
            return None;
        }

        return Some(core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            self.ptr.cast(),
            self.len,
        )));
    }
}

pub unsafe extern "C" fn w2s_string_clone(str: w2s_string) -> w2s_string {
    let byte_len = str.len + 1;
    let mut result = Vec::with_capacity(byte_len);
    core::ptr::copy_nonoverlapping(str.ptr, result.as_mut_ptr(), byte_len);

    let result = ManuallyDrop::new(result.into_boxed_slice());
    return w2s_string {
        ptr: result.as_ptr(),
        len: str.len,
    };
}

pub unsafe extern "C" fn w2s_string_destroy(str: w2s_string) {
    if str.ptr.is_null() {
        return;
    }

    drop(Box::from_raw(core::slice::from_raw_parts_mut(
        str.ptr.cast_mut(),
        str.len + 1,
    )));
}
