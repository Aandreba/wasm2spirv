use crate::{
    ast::function::FunctionConfig,
    error::{Error, Result},
};
use rspirv::spirv::{Capability, MemoryModel};
use std::collections::BTreeMap;
use wasmparser::WasmFeatures;

#[derive(Debug, Clone)]
pub struct ConfigBuilder {
    inner: Config,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub capabilities: CapabilityModel,
    pub functions: BTreeMap<u32, FunctionConfig>,
}

impl Config {
    pub fn set_addressing_model(&mut self, addressing_model: AddressingModel) -> Result<&mut Self> {
        match addressing_model {
            AddressingModel::Logical => {}
            AddressingModel::Physical => self.capabilities.require(
                Capability::Addresses,
                "This addressing model requires the `Addresses` capability",
            )?,
            AddressingModel::PhysicalStorageBuffer => self.capabilities.require(
                Capability::PhysicalStorageBufferAddresses,
                "This addressing model requires the `PhysicalStorageBufferAddresses` capability",
            )?,
        }

        self.addressing_model = addressing_model;
        Ok(self)
    }

    pub fn set_memory_model(&mut self, memory_model: MemoryModel) -> Result<&mut Self> {
        match memory_model {
            MemoryModel::Simple | MemoryModel::GLSL450 => self.capabilities.require(
                Capability::Shader,
                "This memory model requires the `Shader` capability",
            )?,
            MemoryModel::OpenCL => self.capabilities.require(
                Capability::Kernel,
                "This memory model requires the `Kernel` capability",
            )?,
            MemoryModel::Vulkan => self.capabilities.require(
                Capability::VulkanMemoryModel,
                "This memory model requires the `VulkanMemoryModel` capability",
            )?,
        }

        self.memory_model = memory_model;
        Ok(self)
    }
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

impl CapabilityModel {
    fn require(&mut self, capability: Capability, err_msg: &'static str) -> Result<()> {
        let res = match self {
            CapabilityModel::Static(x) => x.contains(&capability),
            CapabilityModel::Dynamic(x) => {
                if !x.contains(&capability) {
                    x.push(capability);
                }
                true
            }
        };

        match res {
            true => Ok(()),
            false => Err(Error::msg(err_msg)),
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

impl Config {
    pub fn builder(
        capabilities: CapabilityModel,
        addressing_model: AddressingModel,
        memory_model: MemoryModel,
    ) -> Result<ConfigBuilder> {
        let mut inner = Config {
            features: WasmFeatures::default(),
            addressing_model,
            memory_model,
            functions: BTreeMap::new(),
            capabilities,
        };

        inner.set_addressing_model(addressing_model)?;
        inner.set_memory_model(memory_model)?;
        return Ok(ConfigBuilder { inner });
    }
}

impl ConfigBuilder {
    pub fn features(&mut self, features: WasmFeatures) -> &mut Self {
        self.inner.features = features;
        self
    }

    pub fn addressing_model(&mut self, addressing_model: AddressingModel) -> Result<&mut Self> {
        self.inner.set_addressing_model(addressing_model)?;
        Ok(self)
    }

    pub fn function(&mut self, f_idx: u32) -> &mut FunctionConfig {
        self.inner
            .functions
            .entry(f_idx)
            .or_insert(FunctionConfig::default())
    }

    pub fn build(&self) -> Config {
        self.inner.clone()
    }
}

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::Dynamic(Vec::new())
    }
}
