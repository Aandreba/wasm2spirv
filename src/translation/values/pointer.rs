use super::{integer::Integer, Value};
use crate::{
    error::{Error, Result},
    r#type::Type,
    translation::module::ModuleTranslator,
};
use rspirv::spirv::{Capability, StorageClass};
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
    pub source: PointerSource,
    pub storage_class: StorageClass,
    pub pointee: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerSource {
    Bitcast,
    AccessChain {
        base: Rc<Pointer>,
        indices: Box<[Rc<Integer>]>,
    },
    PtrAccessChain {
        base: Rc<Pointer>,
        element: Rc<Integer>,
        indices: Box<[Rc<Integer>]>,
    },
    Variable {
        init: Option<Value>,
    },
}

impl Pointer {
    pub fn new_variable(storage_class: StorageClass, pointee: Type) -> Self {
        return Self {
            source: PointerSource::Variable { init: None },
            storage_class,
            pointee,
        };
    }

    pub fn cast(self: &Rc<Self>, new_pointee: Type) -> Pointer {
        return Pointer {
            source: PointerSource::Bitcast,
            storage_class: self.storage_class,
            pointee: new_pointee,
        };
    }

    /// https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#OpAccessChain
    pub fn access_chain(
        self: &Rc<Self>,
        indices: impl IntoIterator<Item = impl Into<Rc<Integer>>>,
    ) -> Result<Pointer> {
        if !self.pointee.is_composite() {
            return Err(Error::msg(format!(
                "Expected a composite pointee type, found '{:?}'",
                self.pointee
            )));
        }

        return Ok(Pointer {
            source: PointerSource::AccessChain {
                base: self.clone(),
                indices: indices.into_iter().map(Into::into).collect(),
            },
            storage_class: self.storage_class,
            pointee: self.pointee.clone(),
        });
    }

    /// https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#OpPtrAccessChain
    pub fn ptr_access_chain(
        self: &Rc<Self>,
        element: impl Into<Rc<Integer>>,
        indices: impl IntoIterator<Item = impl Into<Rc<Integer>>>,
        module: &mut ModuleTranslator,
    ) -> Result<Pointer> {
        self.require_addressing(module)?;
        return Ok(Pointer {
            source: PointerSource::PtrAccessChain {
                base: self.clone(),
                element: element.into(),
                indices: indices.into_iter().map(Into::into).collect(),
            },
            storage_class: self.storage_class,
            pointee: self.pointee.clone(),
        });
    }

    pub fn physical_bytes(&self, module: &ModuleTranslator) -> Option<u32> {
        return module.spirv_address_bytes(self.storage_class);
    }

    fn require_addressing(&self, module: &mut ModuleTranslator) -> Result<()> {
        match self.storage_class {
            StorageClass::PhysicalStorageBuffer => {
                module.require_capability(Capability::PhysicalStorageBufferAddresses)
            }
            _ => module.require_capability(Capability::Addresses),
        }
    }
}
