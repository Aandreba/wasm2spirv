use crate::{error::handle_error, w2s_config, w2s_config_builder};
use spirv::Capability;
use wasm2spirv::{
    capabilities,
    config::{CapabilityModel, Config},
    version::{TargetPlatform, Version},
};

pub type w2c_version = Version;

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
) -> w2s_config_builder {
    let platform = TargetPlatform::from(target);
    let capabilities = CapabilityModel::from(capabilities);

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
