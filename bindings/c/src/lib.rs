#![allow(non_camel_case_types)]

use crate::error::handle_error;
use std::ffi::c_void;
use string::{w2s_byte_view, w2s_string, w2s_string_view, w2s_word_view};
use wasm2spirv::{
    config::{Config, ConfigBuilder},
    Compilation,
};

pub mod config;
pub mod error;
pub mod string;

pub type w2s_compilation = *mut Compilation;

/// Takes ownership of `config`
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

pub unsafe extern "C" fn w2s_compilation_assembly(comp: w2s_compilation) -> w2s_string_view {
    let comp = &*comp;
    if let Some(assembly) = handle_error(comp.assembly()) {
        return w2s_string_view::new_str(assembly);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_words(comp: w2s_compilation) -> w2s_word_view {
    let comp = &*comp;
    if let Some(words) = handle_error(comp.words()) {
        return w2s_word_view::new(words);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_bytes(comp: w2s_compilation) -> w2s_byte_view {
    let comp = &*comp;
    if let Some(bytes) = handle_error(comp.bytes()) {
        return w2s_byte_view::new(bytes);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_glsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(glsl) = handle_error(comp.glsl()) {
        return w2s_string::new(glsl);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_hlsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(hlsl) = handle_error(comp.hlsl()) {
        return w2s_string::new(hlsl);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_msl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(msl) = handle_error(comp.msl()) {
        return w2s_string::new(msl);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_wgsl(comp: w2s_compilation) -> w2s_string {
    let comp = &*comp;
    if let Some(wgsl) = handle_error(comp.wgsl()) {
        return w2s_string::new(wgsl);
    }
    return core::mem::zeroed();
}

pub unsafe extern "C" fn w2s_compilation_destroy(comp: w2s_compilation) {
    drop(Box::from_raw(comp))
}
