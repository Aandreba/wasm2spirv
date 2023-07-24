#![allow(non_upper_case_globals)]

use crate::{
    error::{Error, Result},
    fg::function::{FunctionConfig, FunctionConfigBuilder},
    version::TargetPlatform,
    Str,
};
use docfg::docfg;
use num_enum::TryFromPrimitive;
use rspirv::spirv::{Capability, MemoryModel, StorageClass};
use serde::{Deserialize, Serialize};
use spirv::ExecutionModel;
use std::cell::RefCell;
use vector_mapp::vec::VecMap;

#[derive(Debug, Clone)]
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
#[repr(u8)]
pub enum MemoryGrowErrorKind {
    /// If a `memory.grow` instruction is found, the compilation will fail
    Hard,
    /// If a `memory.grow` instruction is found, it will always return -1 (as per [spec](https://webassembly.github.io/spec/core/syntax/instructions.html#syntax-instr-memory))
    #[default]
    Soft,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, TryFromPrimitive, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
#[repr(u16)]
pub enum AddressingModel {
    #[default]
    Logical,
    Physical,
    PhysicalStorageBuffer,
}

impl AddressingModel {
    pub fn required_capabilities(self) -> Vec<Capability> {
        match self {
            AddressingModel::Logical => Vec::new(),
            AddressingModel::Physical => vec![Capability::Addresses],
            AddressingModel::PhysicalStorageBuffer => {
                vec![Capability::PhysicalStorageBufferAddresses]
            }
        }
    }
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

    pub fn enable_capabilities(&mut self) -> Result<()> {
        let mut capabilities = self.addressing_model.required_capabilities();
        capabilities.extend(memory_model_capabilities(self.memory_model));
        capabilities.extend(
            self.functions
                .values()
                .flat_map(|x| x.required_capabilities()),
        );

        capabilities
            .iter()
            .copied()
            .try_for_each(|x| self.capabilities.require_mut(x))
    }
}

impl ConfigBuilder {
    /// Assert that capability is (or can be) enabled, enabling it if required (and possible).
    pub fn require_capability(&mut self, capability: Capability) -> Result<()> {
        self.inner.capabilities.require_mut(capability)
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

    pub fn set_memory_grow_error(&mut self, memory_grow_error: MemoryGrowErrorKind) -> &mut Self {
        self.inner.memory_grow_error = memory_grow_error;
        self
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

    pub fn build(&self) -> Result<Config> {
        let mut res = self.inner.clone();
        res.enable_capabilities()?;
        Ok(res)
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
        Self::dynamic(vec![Capability::Int64, Capability::Float64])
    }
}

#[docfg(feature = "spirv-tools")]
impl From<&Config> for spirv_tools::val::ValidatorOptions {
    fn from(_: &Config) -> Self {
        return Self { ..Self::default() };
    }
}

pub fn storage_class_capabilities(storage_class: StorageClass) -> Vec<Capability> {
    return match storage_class {
        StorageClass::Uniform
        | StorageClass::Output
        | StorageClass::Private
        | StorageClass::PushConstant
        | StorageClass::StorageBuffer => vec![Capability::Shader],
        StorageClass::PhysicalStorageBuffer => vec![Capability::PhysicalStorageBufferAddresses],
        StorageClass::AtomicCounter => vec![Capability::AtomicStorage],
        StorageClass::Generic => vec![Capability::GenericPointer],
        _ => return Vec::new(),
    };
}

pub fn memory_model_capabilities(memory_model: MemoryModel) -> Vec<Capability> {
    match memory_model {
        MemoryModel::Simple | MemoryModel::GLSL450 => vec![Capability::Shader],
        MemoryModel::OpenCL => vec![Capability::Kernel],
        MemoryModel::Vulkan => vec![Capability::VulkanMemoryModel],
    }
}

pub fn execution_model_capabilities(execution_model: ExecutionModel) -> Vec<Capability> {
    match execution_model {
        ExecutionModel::Fragment | ExecutionModel::GLCompute | ExecutionModel::Vertex => {
            vec![Capability::Shader]
        }
        ExecutionModel::TessellationEvaluation | ExecutionModel::TessellationControl => {
            vec![Capability::Tessellation]
        }
        ExecutionModel::Geometry => vec![Capability::Geometry],
        ExecutionModel::Kernel => vec![Capability::Kernel],
        _ => Vec::new(),
    }
}
