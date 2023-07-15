use rspirv::spirv::Capability;

#[derive(Debug, Clone, PartialEq, Hash, Default)]
pub struct Config {
    adressing_model: AddressingModel,
    capabilities: CapabilityMethod,
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
