use crate::{error::handle_error, string::w2s_string_view, w2s_config, w2s_config_builder};
use spirv::{Capability, MemoryModel};
use wasm2spirv::{
    capabilities,
    config::{AddressingModel, CapabilityModel, Config},
    version::{TargetPlatform, Version},
};

pub type w2c_version = Version;
pub type w2s_config_builder = *mut ConfigBuilder;
pub type w2s_config = *mut Config;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct w2c_target {
    platform: w2s_target_platform,
    version: w2c_version,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(C)]
pub struct w2c_capabilities {
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

pub unsafe extern "C" fn w2s_config_builder_new(
    target: w2c_target,
    capabilities: w2c_capabilities,
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

pub unsafe extern "C" fn w2s_config_builder_destroy(builder: w2s_config_builder) {
    drop(Box::from_raw(builder))
}

impl From<w2c_target> for TargetPlatform {
    fn from(value: w2c_target) -> Self {
        match value.platform {
            w2s_target_platform::Universal => Self::Universal(value.version),
            w2s_target_platform::Vulkan => Self::Vulkan(value.version),
        }
    }
}

impl From<w2c_capabilities> for CapabilityModel {
    fn from(value: w2c_capabilities) -> Self {
        let capabilities =
            unsafe { core::slice::from_raw_parts(value.capabilities, value.capabilities_len) };

        match value.model {
            w2s_capability_model::Static => CapabilityModel::Static(Box::from(capabilities)),
            w2s_capability_model::Dynamic => CapabilityModel::Dynamic(Vec::from(capabilities)),
        }
    }
}
