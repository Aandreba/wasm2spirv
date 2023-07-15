use super::{
    float::Float,
    integer::{Integer, IntegerSource},
    schrodinger::Schrodinger,
    Value,
};
use crate::{
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
    translation::{module::ModuleBuilder, values::float::FloatSource, Operation},
};
use rspirv::spirv::{Capability, StorageClass};
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
    pub source: PointerSource,
    pub storage_class: StorageClass,
    pub pointee: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerSource {
    Casted,
    FromInteger(Rc<Integer>),
    Loaded {
        pointer: Rc<Pointer>,
    },
    FunctionCall {
        args: Box<[Value]>,
    },
    AccessChain {
        base: Rc<Pointer>,
        byte_indices: Box<[Rc<Integer>]>,
    },
    PtrAccessChain {
        base: Rc<Pointer>,
        byte_element: Rc<Integer>,
        byte_indices: Box<[Rc<Integer>]>,
    },
    Variable {
        init: Option<Value>,
    },
}

impl Pointer {
    pub fn new_variable(storage_class: StorageClass, ty: Type) -> Self {
        return Self {
            source: PointerSource::Variable { init: None },
            storage_class,
            pointee: ty,
        };
    }

    pub fn pointer_type(&self) -> Type {
        return Type::Pointer(self.storage_class, Box::new(self.pointee.clone()));
    }

    /// Tyoe of element expected/returned when executing a store/load
    pub fn element_type(&self) -> Type {
        match &self.pointee {
            Type::Scalar(x) => Type::Scalar(*x),
            Type::Composite(CompositeType::StructuredArray(elem)) => Type::Scalar(**elem),
            other @ (Type::Pointer(_, _) | Type::Schrodinger) => other.clone(),
        }
    }

    pub fn new_variable_with_init(
        storage_class: StorageClass,
        ty: Type,
        init: impl Into<Value>,
    ) -> Self {
        return Self {
            source: PointerSource::Variable {
                init: Some(init.into()),
            },
            storage_class,
            pointee: ty,
        };
    }

    pub fn to_integer(self: Rc<Self>, module: &mut ModuleBuilder) -> Result<Integer> {
        self.require_addressing(module)?;
        return Ok(Integer {
            source: IntegerSource::Conversion(super::integer::ConversionSource::FromPointer(self)),
        });
    }

    pub fn cast(&self, new_pointee: Type) -> Pointer {
        return Pointer {
            source: PointerSource::Casted,
            storage_class: self.storage_class,
            pointee: new_pointee,
        };
    }

    pub fn load(self: Rc<Self>, module: &mut ModuleBuilder) -> Result<Value> {
        return Ok(match &self.pointee {
            Type::Pointer(storage_class, pointee) => {
                self.require_variable_pointers(module)?;
                Value::Pointer(Rc::new(Pointer {
                    storage_class: *storage_class,
                    pointee: Type::clone(pointee),
                    source: PointerSource::Loaded { pointer: self },
                }))
            }
            Type::Scalar(ScalarType::I32 | ScalarType::I64) => Value::Integer(Rc::new(Integer {
                source: IntegerSource::Loaded { pointer: self },
            })),
            Type::Scalar(ScalarType::F32 | ScalarType::F64) => Value::Float(Rc::new(Float {
                source: FloatSource::Loaded { pointer: self },
            })),
            Type::Composite(CompositeType::StructuredArray(_)) => {
                return self
                    .access(Integer::new_constant_isize(0, module), module)
                    .map(Rc::new)?
                    .load(module)
            }
            Type::Schrodinger => Value::Schrodinger(Rc::new(Schrodinger {
                source: super::schrodinger::SchrodingerSource::Loaded { pointer: self },
                integer: Cell::new(None),
                pointer: Cell::new(None),
            })),
        });
    }

    pub fn store(self: Rc<Self>, value: Value, module: &mut ModuleBuilder) -> Result<Operation> {
        return Ok(match self.pointee {
            Type::Composite(CompositeType::StructuredArray(_)) => {
                return self
                    .access(Integer::new_constant_isize(0, module), module)
                    .map(Rc::new)?
                    .store(value, module)
            }
            _ => Operation::Store {
                pointer: self,
                value,
            },
        });
    }

    /// Operation executed, depending on pointee type:
    ///     - [`StructuredArray`](CompositeType::StructuredArray): Goes through internal runtime array (via [`access_chain`]).
    ///     - Otherwise, perform [`ptr_access_chain`]
    pub fn access(
        self: Rc<Self>,
        byte_element: impl Into<Rc<Integer>>,
        module: &mut ModuleBuilder,
    ) -> Result<Pointer> {
        match self.pointee {
            Type::Composite(CompositeType::StructuredArray(_)) => {
                let zero = Rc::new(Integer::new_constant_u32(0)); // we need to go through the struct first
                self.access_chain([zero, byte_element.into()])
            }
            _ => self.ptr_access_chain(byte_element, None::<Rc<Integer>>, module),
        }
    }

    /// https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#OpAccessChain
    pub fn access_chain(
        self: Rc<Self>,
        byte_indices: impl IntoIterator<Item = impl Into<Rc<Integer>>>,
    ) -> Result<Pointer> {
        let new_pointee = match &self.pointee {
            Type::Composite(CompositeType::StructuredArray(elem)) => Type::Scalar(**elem),
            _ => {
                return Err(Error::msg(format!(
                    "Expected a composite pointee type, found '{:?}'",
                    self.pointee
                )))
            }
        };

        return Ok(Pointer {
            storage_class: self.storage_class,
            pointee: new_pointee,
            source: PointerSource::AccessChain {
                base: self,
                byte_indices: byte_indices.into_iter().map(Into::into).collect(),
            },
        });
    }

    /// https://registry.khronos.org/SPIR-V/specs/unified1/SPIRV.html#OpPtrAccessChain
    pub fn ptr_access_chain(
        self: Rc<Self>,
        byte_element: impl Into<Rc<Integer>>,
        byte_indices: impl IntoIterator<Item = impl Into<Rc<Integer>>>,
        module: &mut ModuleBuilder,
    ) -> Result<Pointer> {
        self.require_addressing(module)?;
        return Ok(Pointer {
            storage_class: self.storage_class,
            pointee: self.pointee.clone(),
            source: PointerSource::PtrAccessChain {
                base: self,
                byte_element: byte_element.into(),
                byte_indices: byte_indices.into_iter().map(Into::into).collect(),
            },
        });
    }

    /// Byte-size of elements pointed too by the pointer.
    pub fn element_bytes(&self, module: &ModuleBuilder) -> Result<u32> {
        return Ok(match &self.pointee {
            Type::Pointer(storage_class, _) => {
                return module
                    .spirv_address_bytes(*storage_class)
                    .ok_or_else(Error::logical_pointer)
            }
            Type::Scalar(x) => x.byte_size(),
            Type::Composite(CompositeType::StructuredArray(elem)) => elem.byte_size(),
            Type::Schrodinger => match module.spirv_address_bytes(self.storage_class) {
                Some(x) => x,
                None => return Err(Error::logical_pointer()),
            },
        });
    }

    pub fn physical_bytes(&self, module: &ModuleBuilder) -> Option<u32> {
        return module.spirv_address_bytes(self.storage_class);
    }

    pub fn require_addressing(&self, module: &mut ModuleBuilder) -> Result<()> {
        match self.storage_class {
            StorageClass::PhysicalStorageBuffer => {
                module.require_capability(Capability::PhysicalStorageBufferAddresses)
            }
            _ => module.require_capability(Capability::Addresses),
        }
    }

    pub fn require_variable_pointers(&self, module: &mut ModuleBuilder) -> Result<()> {
        match self.storage_class {
            StorageClass::StorageBuffer | StorageClass::PhysicalStorageBuffer => {
                module.require_capability(Capability::VariablePointersStorageBuffer)
            }
            _ => module.require_capability(Capability::VariablePointers),
        }
    }
}
