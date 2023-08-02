#![allow(non_upper_case_globals)]

use crate::{
    error::{Error, Result},
    fg::function::{FunctionConfig, FunctionConfigBuilder},
    version::TargetPlatform,
    Str,
};
use docfg::docfg;
use num_enum::TryFromPrimitive;
use rspirv::spirv::{Capability, MemoryModel};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use vector_mapp::vec::VecMap;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct ConfigBuilder {
    pub(crate) inner: Config,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Config {
    pub platform: TargetPlatform,
    #[serde(default)]
    pub features: WasmFeatures,
    pub addressing_model: AddressingModel,
    pub memory_model: MemoryModel,
    pub capabilities: CapabilityModel,
    pub extensions: Box<[Str<'static>]>,
    #[serde(default)]
    pub memory_grow_error: MemoryGrowErrorKind,
    #[serde(default)]
    pub functions: VecMap<u32, FunctionConfig>,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, TryFromPrimitive, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[repr(i32)]
pub enum MemoryGrowErrorKind {
    /// If a `memory.grow` instruction is found, the compilation will fail
    Hard = 0,
    /// If a `memory.grow` instruction is found, it will always return -1 (as per [spec](https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory))
    #[default]
    Soft = 1,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, TryFromPrimitive, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[repr(u32)]
pub enum AddressingModel {
    #[default]
    Logical = 0,
    Physical = 1,
    PhysicalStorageBuffer = 2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityModel {
    /// The compilation will fail if a required capability isn't manually enabled
    Static(#[serde(default)] Box<[Capability]>),
    /// The compiler may add new capabilities whenever required.
    Dynamic(#[serde(default)] RefCell<Vec<Capability>>),
}

impl CapabilityModel {
    pub fn dynamic(values: impl Into<Vec<Capability>>) -> Self {
        return Self::Dynamic(RefCell::new(values.into()));
    }

    pub fn iter(&mut self) -> std::slice::Iter<'_, Capability> {
        match self {
            CapabilityModel::Static(x) => x.iter(),
            CapabilityModel::Dynamic(x) => x.get_mut().iter(),
        }
    }

    pub fn require(&self, capability: Capability) -> Result<()> {
        match self {
            CapabilityModel::Static(x) => {
                if !x.contains(&capability) {
                    return Err(Error::msg(format!("Unable to enable {capability:?}")));
                }
            }
            CapabilityModel::Dynamic(x) => {
                let mut x = x.borrow_mut();
                if !x.contains(&capability) {
                    x.push(capability);
                }
            }
        }
        Ok(())
    }

    pub fn require_mut(&mut self, capability: Capability) -> Result<()> {
        match self {
            CapabilityModel::Static(x) => {
                if !x.contains(&capability) {
                    return Err(Error::msg(format!("Unable to enable {capability:?}")));
                }
            }
            CapabilityModel::Dynamic(x) => {
                let x = x.get_mut();
                if !x.contains(&capability) {
                    x.push(capability);
                }
            }
        }
        Ok(())
    }
}

impl IntoIterator for CapabilityModel {
    type Item = Capability;
    type IntoIter = std::vec::IntoIter<Capability>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            CapabilityModel::Static(x) => x.into_vec().into_iter(),
            CapabilityModel::Dynamic(x) => x.into_inner().into_iter(),
        }
    }
}

impl Config {
    pub fn builder(
        platform: TargetPlatform,
        capabilities: CapabilityModel,
        extensions: impl IntoIterator<Item = impl Into<Str<'static>>>,
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
            extensions: extensions.into_iter().map(Into::into).collect(),
            memory_grow_error: Default::default(),
        };

        return Ok(ConfigBuilder { inner });
    }
}

impl ConfigBuilder {
    pub fn set_addressing_model(mut self, addressing_model: AddressingModel) -> Self {
        self.inner.addressing_model = addressing_model;
        Ok(self)
    }

    pub fn set_memory_model(mut self, memory_model: MemoryModel) -> Self {
        self.inner.memory_model = memory_model;
        Ok(self)
    }

    pub fn set_memory_grow_error(mut self, memory_grow_error: MemoryGrowErrorKind) -> Self {
        self.inner.memory_grow_error = memory_grow_error;
        self
    }

    pub fn set_features(mut self, features: WasmFeatures) -> Self {
        self.inner.features = features;
        self
    }

    pub fn function(self, f_idx: u32) -> FunctionConfigBuilder {
        return FunctionConfigBuilder {
            inner: Default::default(),
            idx: f_idx,
            config: self,
        };
    }

    pub fn append_functions(mut self, f: impl IntoIterator<Item = (u32, FunctionConfig)>) -> Self {
        self.inner.functions.extend(f);
        self
    }

    pub fn build(self) -> Config {
        return self.inner;
    }
}

impl ConfigBuilder {
    pub fn set_memory_grow_error_boxed(
        mut self: Box<Self>,
        memory_grow_error: MemoryGrowErrorKind,
    ) -> Box<Self> {
        self.inner.memory_grow_error = memory_grow_error;
        self
    }

    pub fn set_features_boxed(mut self: Box<Self>, features: WasmFeatures) -> Box<Self> {
        self.inner.features = features;
        self
    }

    pub fn append_functions_boxed(
        mut self: Box<Self>,
        f: impl IntoIterator<Item = (u32, FunctionConfig)>,
    ) -> Box<Self> {
        self.inner.functions.extend(f);
        self
    }

    pub fn build_boxed(self: Box<Self>) -> Box<Config> {
        return unsafe { Box::from_raw(Box::into_raw(self).cast()) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[repr(C)]
pub struct WasmFeatures {
    pub memory64: bool,
    pub saturating_float_to_int: bool,
}

impl Into<wasmparser::WasmFeatures> for WasmFeatures {
    fn into(self) -> wasmparser::WasmFeatures {
        return wasmparser::WasmFeatures {
            memory64: self.memory64,
            saturating_float_to_int: self.saturating_float_to_int,
            ..Default::default()
        };
    }
}

impl Default for CapabilityModel {
    fn default() -> Self {
        Self::dynamic(vec![Capability::Int64, Capability::Float64])
    }
}

#[docfg(feature = "spirv-tools")]
impl From<&Config> for spirv_tools::val::ValidatorOptions {
    fn from(_: &Config) -> Self {
        return Self { ..Self::default() };
    }
}
