#![allow(non_camel_case_types)]

use std::alloc::Layout;

use crate::error::handle_error;
use config::w2s_config;
use libc::c_void;
use string::{w2s_byte_view, w2s_string, w2s_string_view, w2s_word_view};
use wasm2spirv::Compilation;

pub mod config;
pub mod error;
pub mod string;

pub type w2s_compilation = *mut Compilation;

#[cfg(all(not(feature = "spvc"), not(feature = "naga")))]
compile_error!("At least one of spvc or naga should be enabled");

pub unsafe extern "C" fn w2s_malloc(size: usize, log2_align: u16) -> *mut c_void {
    let layout = Layout::from_size_align_unchecked(size, 1 << align);

    let ptr = std::alloc::alloc(layout);
    if ptr.is_null() {
        std::alloc::handle_alloc_error(layout);
    }
    return ptr.cast();
}

pub unsafe extern "C" fn w2s_free(ptr: *mut c_void, size: usize, log2_align: u16) {
    let layout = Layout::from_size_align_unchecked(size, 1 << align);
    return std::alloc::dealloc(ptr.cast(), layout);
}

/// Takes ownership of `config`
#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_new(
    config: w2s_config,
    bytes: *const u8,
    bytes_len: usize,
) -> w2s_compilation {
    let config = *Box::from_raw(config);
    let bytes = core::slice::from_raw_parts(bytes, bytes_len);

    if let Some(result) = handle_error(Compilation::new(config, bytes)) {
        return Box::into_raw(Box::new(result));
    }
    return core::ptr::null_mut();
}

#[cfg(feature = "spvt")]
#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_optimized(comp: w2s_compilation) -> w2s_compilation {
    let comp = &*comp;
    if let Some(optimized) = handle_error(comp.optimized()) {
        return Box::into_raw(Box::new(optimized));
    }
    return core::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_assembly(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(assembly) = handle_error(comp.assembly()) {
        return w2s_string::new(assembly);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_words(comp: w2s_compilation) -> w2s_word_view {
    let comp = &*comp;
    if let Some(words) = handle_error(comp.words()) {
        return w2s_word_view::new(words);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_bytes(comp: w2s_compilation) -> w2s_byte_view {
    let comp = &*comp;
    if let Some(bytes) = handle_error(comp.bytes()) {
        return w2s_byte_view::new(bytes);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_glsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(glsl) = handle_error(comp.glsl()) {
        return w2s_string::new(glsl);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_hlsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(hlsl) = handle_error(comp.hlsl()) {
        return w2s_string::new(hlsl);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_msl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(msl) = handle_error(comp.msl()) {
        return w2s_string::new(msl);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_wgsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(wgsl) = handle_error(comp.wgsl()) {
        return w2s_string::new(wgsl);
    }
    return core::mem::zeroed();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_compilation_destroy(comp: w2s_compilation) {
    drop(Box::from_raw(comp))
}
