use crate::{
    ast::function::{FunctionConfig, FunctionConfigBuilder},
    error::{Error, Result},
    version::TargetPlatform,
    Str,
};
use num_enum::TryFromPrimitive;
use rspirv::spirv::{Capability, MemoryModel, StorageClass};
use serde::{Deserialize, Serialize};
use std::ops::Deref;
use vector_mapp::vec::VecMap;

#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    pub(crate) inner: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub platform: TargetPlatform,
    #[serde(default)]
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub capabilities: CapabilityModel,
    pub extensions: ExtensionModel,
    #[serde(default)]
    pub functions: VecMap<u32, FunctionConfig>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, TryFromPrimitive, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[repr(u16)]
pub enum AddressingModel {
    #[default]
    Logical,
    Physical,
    PhysicalStorageBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityModel {
    /// The compilation will fail if a required capability isn't manually enabled
    Static(#[serde(default)] Box<[Capability]>),
    /// The compiler may add new capabilities whenever required.
    Dynamic(#[serde(default)] Vec<Capability>),
}

impl Deref for CapabilityModel {
    type Target = [Capability];

    fn deref(&self) -> &Self::Target {
        match self {
            CapabilityModel::Static(x) => x,
            CapabilityModel::Dynamic(x) => x,
        }
    }
}

impl IntoIterator for CapabilityModel {
    type Item = Capability;
    type IntoIter = std::vec::IntoIter<Capability>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CapabilityModel::Static(x) => x.into_vec().into_iter(),
            CapabilityModel::Dynamic(x) => x.into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a CapabilityModel {
    type Item = &'a Capability;
    type IntoIter = std::slice::Iter<'a, Capability>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CapabilityModel::Static(x) => x.iter(),
            CapabilityModel::Dynamic(x) => x.iter(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionModel {
    /// The compilation will fail if a required extension isn't manually enabled
    Static(#[serde(default)] Box<[Str<'static>]>),
    /// The compiler may add new extensions whenever required.
    Dynamic(#[serde(default)] Vec<Str<'static>>),
}

impl ExtensionModel {
    pub fn r#static(iter: impl IntoIterator<Item = impl Into<Str<'static>>>) -> Self {
        Self::Static(iter.into_iter().map(Into::into).collect())
    }

    pub fn dynamic(iter: impl IntoIterator<Item = impl Into<Str<'static>>>) -> Self {
        Self::Dynamic(iter.into_iter().map(Into::into).collect())
    }
}

impl Deref for ExtensionModel {
    type Target = [Str<'static>];

    fn deref(&self) -> &Self::Target {
        match self {
            ExtensionModel::Static(x) => x,
            ExtensionModel::Dynamic(x) => x,
        }
    }
}

impl IntoIterator for ExtensionModel {
    type Item = Str<'static>;
    type IntoIter = std::vec::IntoIter<Str<'static>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ExtensionModel::Static(x) => x.into_vec().into_iter(),
            ExtensionModel::Dynamic(x) => x.into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a ExtensionModel {
    type Item = &'a Str<'static>;
    type IntoIter = std::slice::Iter<'a, Str<'static>>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            ExtensionModel::Static(x) => x.iter(),
            ExtensionModel::Dynamic(x) => x.iter(),
        }
    }
}

impl Default for ExtensionModel {
    fn default() -> Self {
        ExtensionModel::Dynamic(Vec::new())
    }
}

impl Config {
    pub fn builder(
        platform: TargetPlatform,
        capabilities: CapabilityModel,
        extensions: ExtensionModel,
        addressing_model: AddressingModel,
        memory_model: MemoryModel,
    ) -> Result<ConfigBuilder> {
        let inner = Config {
            platform,
            features: WasmFeatures::default(),
            addressing_model,
            memory_model,
            functions: VecMap::new(),
            capabilities,
            extensions,
        };

        let mut builder = ConfigBuilder { inner };
        builder.set_addressing_model(addressing_model)?;
        builder.set_memory_model(memory_model)?;
        return Ok(builder);
    }
}

impl ConfigBuilder {
    /// Assert that capability is (or can be) enabled, enabling it if required (and possible).
    pub fn require_capability(&mut self, capability: Capability) -> Result<()> {
        return match self.inner.capabilities {
            CapabilityModel::Static(ref cap) if cap.contains(&capability) => Ok(()),
            CapabilityModel::Dynamic(ref mut cap) => {
                if !cap.contains(&capability) {
                    cap.push(capability)
                }
                Ok(())
            }
            CapabilityModel::Static(_) => {
                return Err(Error::msg(format!(
                    "Capability '{capability:?}' is not enabled"
                )))
            }
        };
    }

    pub fn set_addressing_model(&mut self, addressing_model: AddressingModel) -> Result<&mut Self> {
        match addressing_model {
            AddressingModel::Logical => {}
            AddressingModel::Physical => self.require_capability(Capability::Addresses)?,
            AddressingModel::PhysicalStorageBuffer => {
                self.require_capability(Capability::PhysicalStorageBufferAddresses)?
            }
        }

        self.inner.addressing_model = addressing_model;
        Ok(self)
    }

    pub fn set_memory_model(&mut self, memory_model: MemoryModel) -> Result<&mut Self> {
        match memory_model {
            MemoryModel::Simple | MemoryModel::GLSL450 => {
                self.require_capability(Capability::Shader)?
            }
            MemoryModel::OpenCL => self.require_capability(Capability::Kernel)?,
            MemoryModel::Vulkan => self.require_capability(Capability::VulkanMemoryModel)?,
        }

        self.inner.memory_model = memory_model;
        Ok(self)
    }

    pub fn set_features(&mut self, features: WasmFeatures) -> &mut Self {
        self.inner.features = features;
        self
    }

    pub fn function<'a>(&'a mut self, f_idx: u32) -> FunctionConfigBuilder<'a> {
        return FunctionConfigBuilder {
            inner: Default::default(),
            idx: f_idx,
            config: self,
        };
    }

    pub fn build(&self) -> Config {
        self.inner.clone()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(packed)]
pub struct WasmFeatures {
    pub memory64: bool,
}

impl WasmFeatures {
    pub fn into_integer(self) -> u64 {
        let mut res = 0;
        if self.memory64 {
            res |= 1 << 0;
        }
        res
    }

    pub fn from_integer(v: u64) -> Self {
        let memory64 = (v & 1) == 1;
        return Self { memory64 };
    }
}

impl Into<wasmparser::WasmFeatures> for WasmFeatures {
    fn into(self) -> wasmparser::WasmFeatures {
        return wasmparser::WasmFeatures {
            memory64: self.memory64,
            ..Default::default()
        };
    }
}

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::Dynamic(vec![Capability::Int64, Capability::Float64])
    }
}

#[cfg(feature = "spirv-tools")]
impl From<&Config> for spirv_tools::val::ValidatorOptions {
    fn from(_: &Config) -> Self {
        return Self { ..Self::default() };
    }
}

pub fn storage_class_capability(storage_class: StorageClass) -> Option<Capability> {
    return Some(match storage_class {
        StorageClass::Uniform
        | StorageClass::Output
        | StorageClass::Private
        | StorageClass::PushConstant
        | StorageClass::StorageBuffer => Capability::Shader,
        StorageClass::PhysicalStorageBuffer => Capability::PhysicalStorageBufferAddresses,
        StorageClass::AtomicCounter => Capability::AtomicStorage,
        StorageClass::Generic => Capability::GenericPointer,
        _ => return None,
    });
}
