use crate::ast::function::FunctionConfig;
use rspirv::spirv::Capability;
use std::collections::BTreeMap;
use wasmparser::WasmFeatures;

#[derive(Debug, Clone, Default)]
pub struct ConfigBuilder {
    inner: Config,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
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

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::Dynamic(Vec::new())
    }
}
