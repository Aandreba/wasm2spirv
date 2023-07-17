use super::{
    float::Float,
    integer::{Integer, IntegerSource},
    Value,
};
use crate::{
    ast::{module::ModuleBuilder, values::float::FloatSource, Operation},
    decorator::VariableDecorator,
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
};
use rspirv::spirv::{Capability, StorageClass};
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub enum Pointee {
    Type(Type),
    /// May be a pointer, may be an integer. You won't know until you load/store.
    Schrodinger,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Pointer {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub source: PointerSource,
    pub storage_class: StorageClass,
    pub pointee: Pointee,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointerSource {
    FunctionParam,
    Casted {
        prev: Rc<Pointer>,
    },
    FromInteger(Rc<Integer>),
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
    FunctionCall {
        args: Box<[Value]>,
    },
    Access {
        base: Rc<Pointer>,
        byte_element: Rc<Integer>,
    },
    Variable {
        init: Option<Value>,
        decorators: Box<[VariableDecorator]>,
    },
}

impl Pointer {
    pub fn new(
        source: PointerSource,
        storage_class: StorageClass,
        pointee: impl Into<Pointee>,
    ) -> Pointer {
        return Self {
            translation: Cell::new(None),
            source,
            storage_class,
            pointee: pointee.into(),
        };
    }

    pub fn new_variable(
        storage_class: StorageClass,
        pointee: impl Into<Pointee>,
        decorators: impl IntoIterator<Item = VariableDecorator>,
    ) -> Self {
        return Self {
            translation: Cell::new(None),
            source: PointerSource::Variable {
                init: None,
                decorators: decorators.into_iter().collect(),
            },
            storage_class,
            pointee: pointee.into(),
        };
    }

    pub fn new_variable_with_init(
        storage_class: StorageClass,
        pointee: impl Into<Pointee>,
        init: impl Into<Value>,
        decorators: impl IntoIterator<Item = VariableDecorator>,
    ) -> Self {
        return Self {
            translation: Cell::new(None),
            source: PointerSource::Variable {
                init: Some(init.into()),
                decorators: decorators.into_iter().collect(),
            },
            storage_class,
            pointee: pointee.into(),
        };
    }

    /// Returns an unsigned 32-bit integer
    pub fn pointee_byte_size(self: Rc<Self>, module: &ModuleBuilder) -> Option<Integer> {
        match &self.pointee {
            Pointee::Type(Type::Pointer(storage_class, _)) => module
                .spirv_address_bytes(*storage_class)
                .map(Integer::new_constant_u32),
            Pointee::Type(Type::Scalar(x)) => Some(Integer::new_constant_u32(x.byte_size())),
            Pointee::Type(Type::Composite(CompositeType::StructuredArray(_))) => Some(Integer {
                translation: Cell::new(None),
                source: IntegerSource::ArrayLength {
                    structured_array: self,
                },
            }),
            Pointee::Schrodinger => todo!(),
        }
    }

    pub fn pointer_type(&self) -> Type {
        return Type::Pointer(self.storage_class, Box::new(self.pointee.clone()));
    }

    /// Tyoe of element expected/returned when executing a store/load/access
    pub fn element_type(&self) -> Type {
        match &self.pointee {
            Type::Composite(CompositeType::StructuredArray(elem)) => Type::Scalar(**elem),
            other => other.clone(),
        }
    }

    pub fn to_integer(self: Rc<Self>, module: &mut ModuleBuilder) -> Result<Integer> {
        self.require_addressing(module)?;
        return Ok(Integer {
            translation: Cell::new(None),
            source: IntegerSource::Conversion(super::integer::ConversionSource::FromPointer(self)),
        });
    }

    pub fn cast(self: Rc<Self>, new_pointee: impl Into<Type>) -> Rc<Pointer> {
        let new_pointee = new_pointee.into();
        if self.pointee == new_pointee {
            return self;
        }

        return Rc::new(Pointer {
            translation: Cell::new(None),
            storage_class: self.storage_class,
            pointee: new_pointee,
            source: PointerSource::Casted { prev: self },
        });
    }

    pub fn load(
        self: Rc<Self>,
        log2_alignment: Option<u32>,
        module: &mut ModuleBuilder,
    ) -> Result<Value> {
        return Ok(match &self.pointee {
            Type::Pointer(storage_class, pointee) => {
                self.require_variable_pointers(module)?;
                Value::Pointer(Rc::new(Pointer {
                    translation: Cell::new(None),
                    storage_class: *storage_class,
                    pointee: Type::clone(pointee),
                    source: PointerSource::Loaded {
                        pointer: self,
                        log2_alignment,
                    },
                }))
            }
            Type::Scalar(ScalarType::I32 | ScalarType::I64) => Value::Integer(Rc::new(Integer {
                translation: Cell::new(None),
                source: IntegerSource::Loaded {
                    pointer: self,
                    log2_alignment,
                },
            })),
            Type::Scalar(ScalarType::F32 | ScalarType::F64) => Value::Float(Rc::new(Float {
                translation: Cell::new(None),
                source: FloatSource::Loaded {
                    pointer: self,
                    log2_alignment,
                },
            })),
            Type::Composite(CompositeType::StructuredArray(_)) => {
                return Rc::new(self.access(Integer::new_constant_isize(0, module), module))
                    .load(log2_alignment, module)
            }
            Type::Schrodinger => Value::Schrodinger(Rc::new(Schrodinger {
                source: super::schrodinger::SchrodingerSource::Loaded { pointer: self },
                integer: Cell::new(None),
                pointer: Cell::new(None),
            })),
        });
    }

    pub fn store(
        self: Rc<Self>,
        value: Value,
        log2_alignment: Option<u32>,
        module: &mut ModuleBuilder,
    ) -> Result<Operation> {
        return Ok(match self.pointee {
            Type::Composite(CompositeType::StructuredArray(_)) => {
                return Rc::new(self.access(Integer::new_constant_isize(0, module), module)).store(
                    value,
                    log2_alignment,
                    module,
                )
            }
            _ => Operation::Store {
                pointer: self,
                value,
                log2_alignment,
            },
        });
    }

    /// Operation executed, depending on pointee type:
    ///     - [`StructuredArray`](CompositeType::StructuredArray) goes through internal runtime array (via [`access_chain`]).
    ///     - Otherwise, perform [`ptr_access_chain`]
    pub fn access(
        self: Rc<Self>,
        byte_element: impl Into<Rc<Integer>>,
        module: &mut ModuleBuilder,
    ) -> Pointer {
        return Self {
            pointee: self.element_type(),
            storage_class: self.storage_class,
            translation: Cell::new(None),
            source: PointerSource::Access {
                base: self,
                byte_element: byte_element.into(),
            },
        };
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

impl From<Type> for Pointee {
    fn from(value: Type) -> Self {
        Self::Type(value)
    }
}
