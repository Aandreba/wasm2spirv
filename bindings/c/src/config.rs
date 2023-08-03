#![feature(array_try_from_fn)]

use crate::{error::handle_error, string::w2s_string_view};
use elor::Either;
use libc::c_void;
use spirv::{Capability, MemoryModel};
use std::{
    borrow::Cow,
    cell::RefCell,
    convert::Infallible,
    io::BufReader,
    mem::ManuallyDrop,
    ops::DerefMut,
    os::fd::{FromRawFd, RawFd},
};
use wasm2spirv::{
    config::{
        AddressingModel, CapabilityModel, Config, ConfigBuilder, MemoryGrowErrorKind, WasmFeatures,
    },
    error::{Error, Result},
    fg::function::{ExecutionMode, FunctionConfig, FunctionConfigBuilder},
    version::{TargetPlatform, Version},
};

pub type w2c_version = Version;
pub type w2s_config_builder = *mut ConfigBuilder;
pub type w2s_config = *mut Config;
pub type w2s_function_config_builder = *mut FunctionConfigBuilder;
pub type w2s_function_config = *mut FunctionConfig;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct w2s_target {
    platform: w2s_target_platform,
    version: w2c_version,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct w2s_capabilities {
    model: w2s_capability_model,
    capabilities: *const Capability,
    capabilities_len: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum w2s_target_platform {
    Universal = 0,
    Vulkan = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum w2s_capability_model {
    Static = 0,
    Dynamic = 1,
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_from_json_string(
    json: *const u8,
    json_len: usize,
) -> w2s_config {
    if let Some(config) = handle_error(
        serde_json::from_slice::<Config>(core::slice::from_raw_parts(json, json_len))
            .map_err(Error::msg),
    ) {
        return Box::into_raw(Box::new(config));
    }
    return core::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_from_json_fd(json: RawFd) -> w2s_config {
    let file = std::fs::File::from_raw_fd(json);
    let mut reader = ManuallyDrop::new(BufReader::new(file));

    if let Some(config) =
        handle_error(serde_json::from_reader::<_, Config>(reader.deref_mut()).map_err(Error::msg))
    {
        return Box::into_raw(Box::new(config));
    }
    return core::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_clone(config: w2s_config) -> w2s_config {
    let config = &*config;
    return Box::into_raw(Box::new(config.clone()));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_new(
    target: w2s_target,
    capabilities: w2s_capabilities,
    extensions: *const w2s_string_view,
    extenrions_len: usize,
    addressing_model: AddressingModel,
    memory_model: MemoryModel,
) -> w2s_config_builder {
    let platform = TargetPlatform::from(target);
    let capabilities = CapabilityModel::from(capabilities);
    let extensions = core::slice::from_raw_parts(extensions, extenrions_len)
        .into_iter()
        .map(|x| Box::<str>::from(x.as_str()));

    if let Some(builder) = handle_error(Config::builder(
        platform,
        capabilities,
        extensions,
        addressing_model,
        memory_model,
    )) {
        return Box::into_raw(Box::new(builder));
    }
    return core::ptr::null_mut();
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_set_memory_grow_error(
    builder: w2s_config_builder,
    kind: MemoryGrowErrorKind,
) {
    ManuallyDrop::new(Box::from_raw(builder).set_memory_grow_error_boxed(kind));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_set_wasm_features(
    builder: w2s_config_builder,
    kind: WasmFeatures,
) {
    ManuallyDrop::new(Box::from_raw(builder).set_features_boxed(kind));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_clone(
    builder: w2s_config_builder,
) -> w2s_config_builder {
    let builder = &*builder;
    return Box::into_raw(Box::new(builder.clone()));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_build(builder: w2s_config_builder) -> w2s_config {
    return Box::into_raw(Box::from_raw(builder).build_boxed());
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_builder_destroy(builder: w2s_config_builder) {
    drop(Box::from_raw(builder))
}

#[no_mangle]
pub unsafe extern "C" fn w2s_config_destroy(config: w2s_config) {
    drop(Box::from_raw(config))
}

/* FUNCTION */
#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_builder_new() -> w2s_function_config_builder {
    return Box::into_raw(Box::new(FunctionConfigBuilder::__new()));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_builder_add_execution_mode(
    builder: w2s_function_config_builder,
    exec_mode: spirv::ExecutionMode,
    data: *const c_void,
    data_len: usize,
) -> bool {
    use spirv::ExecutionMode::*;

    let data = core::slice::from_raw_parts(data.cast::<u8>(), data_len);
    let exec_mode = match exec_mode {
        Invocations => match handle_error(TryInto::<[u8; 4]>::try_into(data).map_err(Error::msg)) {
            Some(bytes) => ExecutionMode::Invocations(u32::from_ne_bytes(bytes)),
            None => return false,
        },
        PixelCenterInteger => ExecutionMode::PixelCenterInteger,
        OriginUpperLeft => ExecutionMode::OriginUpperLeft,
        OriginLowerLeft => ExecutionMode::OriginLowerLeft,
        DepthReplacing => ExecutionMode::DepthReplacing,
        LocalSize => match handle_error(words_from_data(data)) {
            Some([x, y, z]) => ExecutionMode::LocalSize(x, y, z),
            None => return false,
        },
        LocalSizeHint => match handle_error(words_from_data(data)) {
            Some([x, y, z]) => ExecutionMode::LocalSizeHint(x, y, z),
            None => return false,
        },
        other => {
            handle_error::<Infallible, _>(Err(Error::msg(format!(
                "Unsupported execution mode '{other:?}'"
            ))));
            return false;
        }
    };

    return true;
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_builder_build(
    builder: w2s_function_config_builder,
) -> w2s_function_config {
    let builder = Box::from_raw(builder);
    let config = builder.__build().unwrap_right_unchecked();
    return Box::into_raw(Box::new(config));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_builder_clone(
    builder: w2s_function_config_builder,
) -> w2s_function_config_builder {
    let builder = &*builder;
    return Box::into_raw(Box::new(builder.clone()));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_clone(
    config: w2s_function_config,
) -> w2s_function_config {
    let config = &*config;
    return Box::into_raw(Box::new(config.clone()));
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_builder_destroy(builder: w2s_function_config_builder) {
    drop(Box::from_raw(builder))
}

#[no_mangle]
pub unsafe extern "C" fn w2s_function_config_destroy(config: w2s_function_config) {
    drop(Box::from_raw(config))
}

impl From<w2s_target> for TargetPlatform {
    fn from(value: w2s_target) -> Self {
        match value.platform {
            w2s_target_platform::Universal => Self::Universal(value.version),
            w2s_target_platform::Vulkan => Self::Vulkan(value.version),
        }
    }
}

impl From<w2s_capabilities> for CapabilityModel {
    fn from(value: w2s_capabilities) -> Self {
        let capabilities =
            unsafe { core::slice::from_raw_parts(value.capabilities, value.capabilities_len) };

        match value.model {
            w2s_capability_model::Static => CapabilityModel::Static(Box::from(capabilities)),
            w2s_capability_model::Dynamic => {
                CapabilityModel::Dynamic(RefCell::new(Vec::from(capabilities)))
            }
        }
    }
}

fn words_from_data<const N: usize>(data: &[u8]) -> Result<[u32; N]> {
    match bytes_to_words(data)? {
        Either::Left(words) => return TryInto::<[u32; N]>::try_into(words).map_err(Error::msg),
        Either::Right(mut iter) => {
            return core::array::try_from_fn(|_| iter.next())
                .ok_or_else(|| Error::msg("Invalid number of words"));
        }
    }
}

fn bytes_to_words<'a>(
    bytes: &'a [u8],
) -> Result<Either<&'a [u32], impl 'a + Iterator<Item = u32>>> {
    if (bytes.len() % 4 != 0) {
        return Err(Error::msg(
            "There isn't a whole number of words in this byte array",
        ));
    }

    let word_count = bytes.len() / 4;
    unsafe {
        if bytes.as_ptr().align_offset(core::mem::align_of::<u32>()) == 0 {
            return Ok(Either::Left(core::slice::from_raw_parts(
                bytes.as_ptr().cast(),
                word_count,
            )));
        } else {
            let words = bytes
                .chunks_exact(4)
                .map(|x| u32::from_ne_bytes(TryInto::<[u8; 4]>::try_into(x).unwrap_unchecked()));
            return Ok(Either::Right(words));
        }
    }
}
