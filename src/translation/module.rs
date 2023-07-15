use crate::{
    config::{CapabilityMethod, Config},
    error::{Error, Result},
};
use rspirv::spirv::{AddressingModel, Capability, StorageClass};
use wasmparser::{FuncType, Validator};

pub struct ModuleTranslator {
    pub capabilities: CapabilityMethod,
    pub addressing_model: AddressingModel,
    pub wasm_memory64: bool,
    pub functions: Box<[FuncType]>,
}

impl ModuleTranslator {
    pub fn new(config: Config, bytes: &[u8]) -> Result<Self> {
        let mut validator = Validator::new_with_features(config.features);
        let types = validator.validate_all(bytes)?;

        let capabilities = config.capabilities.clone();
        let wasm_memory64 = types.memory_at(0).memory64;
        let addressing_model = match (config.adressing_model, wasm_memory64) {
            (crate::config::AddressingModel::Logical, _) => AddressingModel::Logical,
            (crate::config::AddressingModel::Physical, false) => AddressingModel::Physical32,
            (crate::config::AddressingModel::Physical, true) => AddressingModel::Physical64,
            (crate::config::AddressingModel::PhysicalStorageBuffer, true) => {
                AddressingModel::PhysicalStorageBuffer64
            }
            _ => return Err(Error::msg("Invalid addressing model")),
        };

        let mut functions = Vec::with_capacity(types.function_count() as usize);
        for i in 0..types.function_count() {
            let f = match types
                .get(types.function_at(i))
                .ok_or_else(Error::unexpected)?
            {
                wasmparser::types::Type::Sub(ty) => match &ty.structural_type {
                    wasmparser::StructuralType::Func(f) => f.clone(),
                    _ => return Err(Error::unexpected()),
                },
                _ => return Err(Error::unexpected()),
            };
            functions.push(f);
        }

        let mut result = Self {
            capabilities,
            addressing_model,
            wasm_memory64,
            functions: functions.into_boxed_slice(),
        };

        return Ok(result);
    }

    /// Assert that capability is (or can be) enabled, enabling it if required (and possible).
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
