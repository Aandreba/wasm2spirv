use crate::ast::function::FunctionConfig;
use rspirv::spirv::{Capability, MemoryModel};
use std::collections::BTreeMap;
use wasmparser::WasmFeatures;

#[derive(Debug, Clone, Default)]
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
    pub fn builder() -> ConfigBuilder {
        return ConfigBuilder::default();
    }
}

impl ConfigBuilder {
    pub fn features(&mut self, features: WasmFeatures) -> &mut Self {
        self.inner.features = features;
        self
    }

    pub fn addressing_model(&mut self, addressing_model: AddressingModel) -> &mut Self {
        self.inner.addressing_model = addressing_model;
        self
    }

    pub fn capability_model(&mut self, capabilities: CapabilityModel) -> &mut Self {
        self.inner.capabilities = capabilities;
        self
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

impl Default for Config {
    fn default() -> Self {
        Self {
            features: Default::default(),
            addressing_model: Default::default(),
            memory_model: MemoryModel::Simple,
            capabilities: Default::default(),
            functions: Default::default(),
        }
    }
}

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::Dynamic(Vec::new())
    }
}
