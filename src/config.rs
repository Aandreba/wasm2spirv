use crate::{
    ast::function::{FunctionConfig, FunctionConfigBuilder},
    error::{Error, Result},
};
use rspirv::spirv::{Capability, MemoryModel, StorageClass};
use std::collections::BTreeMap;
use wasmparser::WasmFeatures;

#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    pub(crate) inner: Config,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub capabilities: CapabilityModel,
    pub functions: BTreeMap<u32, FunctionConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressingModel {
    #[default]
    Logical,
    Physical,
    PhysicalStorageBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl Config {
    pub fn builder(
        capabilities: CapabilityModel,
        addressing_model: AddressingModel,
        memory_model: MemoryModel,
    ) -> Result<ConfigBuilder> {
        let inner = Config {
            features: WasmFeatures::default(),
            addressing_model,
            memory_model,
            functions: BTreeMap::new(),
            capabilities,
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
