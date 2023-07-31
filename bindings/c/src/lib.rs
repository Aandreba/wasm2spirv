use std::ffi::c_void;
use wasm2spirv::{config::Config, Compilation};

pub mod error;
pub mod string;

pub type w2s_compilation_config_t = *mut Config;
pub type w2s_compilation_t = *mut Compilation;

/// Takes ownership of `config`
pub unsafe extern "C" fn w2s_compilation_new(
    config: w2s_compilation_config_t,
    bytes: *const u8,
    bytes_len: usize,
) -> w2s_compilation_t {
    let config = *Box::from_raw(config);
    let bytes = core::slice::from_raw_parts(bytes, bytes_len);

    let result = Compilation::new(config, bytes);

    todo!()
}
