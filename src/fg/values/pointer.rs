use super::{
    bool::{Bool, BoolSource},
    float::{Float, FloatSource},
    integer::{Integer, IntegerSource},
    vector::{Vector, VectorSource},
    Value,
};
use crate::{
    decorator::VariableDecorator,
    error::Result,
    fg::{block::BlockBuilder, function::Storeable, module::ModuleBuilder, Operation},
    r#type::{CompositeType, PointerSize, ScalarType, Type},
};
use spirv::StorageClass;
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone)]
pub enum PointerKind {
    Skinny {
        translation: Cell<Option<rspirv::spirv::Word>>,
    },
    Fat {
        translation: Rc<Cell<Option<rspirv::spirv::Word>>>,
        byte_offset: Option<Rc<Integer>>,
    },
}

impl PointerKind {
    pub fn skinny() -> Self {
        Self::Skinny {
            translation: Cell::default(),
        }
    }

    pub fn fat() -> Self {
        Self::Fat {
            translation: Rc::default(),
            byte_offset: None,
        }
    }

    pub fn to_pointer_size(&self) -> PointerSize {
        match self {
            PointerKind::Skinny { .. } => PointerSize::Skinny,
            PointerKind::Fat { .. } => PointerSize::Fat,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pointer {
    pub kind: PointerKind,
    pub storage_class: StorageClass,
    pub pointee: Type,
    pub source: PointerSource,
}

impl Pointer {
    pub fn new(
        kind: PointerKind,
        storage_class: StorageClass,
        pointee: impl Into<Type>,
        source: PointerSource,
    ) -> Self {
        return Self {
            kind,
            source,
            storage_class,
            pointee: pointee.into(),
        };
    }

    pub fn new_variable(
        size: PointerSize,
        storage_class: StorageClass,
        ty: impl Into<Type>,
        init: Option<Value>,
        decorators: impl Into<Box<[VariableDecorator]>>,
    ) -> Self {
        return Self::new(
            size.to_pointer_kind(),
            storage_class,
            ty,
            PointerSource::Variable {
                init,
                decorators: decorators.into(),
            },
        );
    }

    pub fn is_fat(&self) -> bool {
        matches!(self.kind, PointerKind::Fat { .. })
    }

    pub fn is_skinny(&self) -> bool {
        matches!(self.kind, PointerKind::Skinny { .. })
    }

    pub fn byte_offset(&self) -> Option<Rc<Integer>> {
        match &self.kind {
            PointerKind::Skinny { .. } => None,
            PointerKind::Fat { byte_offset, .. } => byte_offset.clone(),
        }
    }

    pub fn cast(self: Rc<Self>, new_pointee: impl Into<Type>) -> Pointer {
        let kind = match self.kind {
            PointerKind::Skinny { .. } => PointerKind::skinny(),
            PointerKind::Fat { .. } => self.kind.clone(),
        };

        return Pointer::new(
            kind,
            self.storage_class,
            new_pointee.into(),
            PointerSource::Casted { prev: self },
        );
    }

    pub fn to_integer(self: Rc<Self>, module: &mut ModuleBuilder) -> Result<Integer> {
        return Ok(Integer {
            translation: Cell::new(None),
            source: IntegerSource::Conversion(super::integer::ConversionSource::FromPointer(self)),
        });
    }

    pub fn store(
        self: Rc<Self>,
        value: impl Into<Value>,
        log2_alignment: Option<u32>,
        _block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<Operation> {
        let value = value.into();

        // TODO If value was just loaded, do a copy instead

        return Ok(Operation::Store {
            target: Storeable::Pointer(self),
            value,
            log2_alignment,
        });
    }

    pub fn load(
        self: Rc<Self>,
        log2_alignment: Option<u32>,
        _block: &mut BlockBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<Value> {
        let result = match &self.pointee {
            Type::Pointer {
                size,
                storage_class,
                pointee,
            } => Value::Pointer(Rc::new(Pointer::new(
                size.to_pointer_kind(),
                *storage_class,
                Type::clone(pointee),
                PointerSource::Loaded {
                    pointer: self,
                    log2_alignment,
                },
            ))),

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

            Type::Scalar(ScalarType::Bool) => Bool::new(BoolSource::Loaded {
                pointer: self,
                log2_alignment,
            })
            .into(),

            Type::Composite(CompositeType::Vector(elem, count)) => Vector {
                translation: Cell::new(None),
                element_type: *elem,
                element_count: *count,
                source: VectorSource::Loaded {
                    pointer: self,
                    log2_alignment,
                },
            }
            .into(),
        };

        return Ok(result);
    }

    pub fn access(
        self: Rc<Self>,
        byte_offset: impl Into<Rc<Integer>>,
        module: &ModuleBuilder,
    ) -> Result<Self> {
        let byte_offset = byte_offset.into();
        let kind = match &self.kind {
            PointerKind::Skinny { translation } => {
                todo!()
            }
            PointerKind::Fat {
                translation,
                byte_offset: offset,
            } => PointerKind::Fat {
                translation: translation.clone(),
                byte_offset: Some(match offset {
                    Some(offset) => offset.clone().add(byte_offset, module)?,
                    None => byte_offset,
                }),
            },
        };

        return Ok(Pointer::new(
            kind,
            self.storage_class,
            self.pointee.clone(),
            self.source.clone(),
        ));
    }

    pub fn physical_bytes(&self, module: &ModuleBuilder) -> Option<u32> {
        return module.spirv_address_bytes(self.storage_class);
    }
}

#[derive(Debug, Clone)]
pub enum PointerSource {
    FunctionParam,
    FromInteger(Rc<Integer>),
    Select {
        selector: Rc<Bool>,
        true_value: Rc<Pointer>,
        false_value: Rc<Pointer>,
    },
    Casted {
        prev: Rc<Pointer>,
    },
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
    Variable {
        init: Option<Value>,
        decorators: Box<[VariableDecorator]>,
    },
}
