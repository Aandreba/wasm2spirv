use rspirv::spirv::Capability;
use wasmparser::WasmFeatures;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub features: WasmFeatures,
    pub adressing_model: AddressingModel,
    pub capabilities: CapabilityMethod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AddressingModel {
    #[default]
    Logical,
    Physical,
    PhysicalStorageBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CapabilityMethod {
    /// The compilation will fail if a required capability isn't manually enabled
    Static(Box<[Capability]>),
    /// The compiler may add new capabilities whenever required.
    Dynamic(Vec<Capability>),
}

impl Default for CapabilityMethod {
    fn default() -> Self {
        Self::Dynamic(Vec::new())
    }
}
