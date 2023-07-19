use crate::{
    ast::function::{FunctionConfig, FunctionConfigBuilder},
    error::{Error, Result},
    version::{SpirvVersion, TargetPlatform},
    Str,
};
use rspirv::spirv::{Capability, MemoryModel, StorageClass};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    pub(crate) inner: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub platform: TargetPlatform,
    pub version: SpirvVersion,
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub capabilities: CapabilityModel,
    pub extensions: ExtensionModel,
    pub functions: BTreeMap<u32, FunctionConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum AddressingModel {
    #[default]
    Logical,
    Physical,
    PhysicalStorageBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CapabilityModel {
    /// The compilation will fail if a required capability isn't manually enabled
    Static(Box<[Capability]>),
    /// The compiler may add new capabilities whenever required.
    Dynamic(Vec<Capability>),
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
pub enum ExtensionModel {
    /// The compilation will fail if a required extension isn't manually enabled
    Static(Box<[Str<'static>]>),
    /// The compiler may add new extensions whenever required.
    Dynamic(Vec<Str<'static>>),
}

impl ExtensionModel {
    pub fn r#static(iter: impl IntoIterator<Item = impl Into<Str<'static>>>) -> Self {
        Self::Static(iter.into_iter().map(Into::into).collect())
    }

    pub fn dynamic(iter: impl IntoIterator<Item = impl Into<Str<'static>>>) -> Self {
        Self::Dynamic(iter.into_iter().map(Into::into).collect())
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

impl Config {
    pub fn builder(
        platform: TargetPlatform,
        version: Option<SpirvVersion>,
        capabilities: CapabilityModel,
        extensions: ExtensionModel,
        addressing_model: AddressingModel,
        memory_model: MemoryModel,
    ) -> Result<ConfigBuilder> {
        let inner = Config {
            version: version.unwrap_or_else(|| platform.into()),
            platform,
            features: WasmFeatures::default(),
            addressing_model,
            memory_model,
            functions: BTreeMap::new(),
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
