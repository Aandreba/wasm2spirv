use crate::{
    config::CapabilityMethod,
    error::{Error, Result},
};
use rspirv::spirv::{AddressingModel, Capability, StorageClass};

pub struct ModuleTranslator {
    pub capabilities: CapabilityMethod,
    pub addressing_model: AddressingModel,
    pub wasm_memory64: bool,
}

impl ModuleTranslator {
    /// Assert that capability is (or can be) enabled, enabling it if required and possible.
    pub fn require_capability(&mut self, capability: Capability) -> Result<()> {
        return match self.capabilities {
            CapabilityMethod::Static(ref cap) if cap.contains(&capability) => Ok(()),
            CapabilityMethod::Dynamic(ref mut cap) => {
                if !cap.contains(&capability) {
                    cap.push(capability)
                }
                Ok(())
            }
            CapabilityMethod::Static(_) => {
                return Err(Error::msg(format!(
                    "Capability '{capability:?}' is not enabled"
                )))
            }
        };
    }

    pub fn spirv_address_bits(&self, storage_class: StorageClass) -> Option<u32> {
        match (storage_class, self.addressing_model) {
            (_, AddressingModel::Physical32) => Some(32),
            (_, AddressingModel::Physical64)
            | (StorageClass::PhysicalStorageBuffer, AddressingModel::PhysicalStorageBuffer64) => {
                Some(64)
            }
            _ => None,
        }
    }

    pub fn spirv_address_bytes(&self, storage_class: StorageClass) -> Option<u32> {
        self.spirv_address_bits(storage_class).map(|x| x / 8)
    }

    pub fn wasm_address_bits(&self) -> u32 {
        match self.wasm_memory64 {
            true => 64,
            false => 32,
        }
    }

    pub fn wasm_address_bytes(&self) -> u32 {
        self.wasm_address_bits() / 8
    }
}
