#![allow(clippy::should_implement_trait)]

use rspirv::spirv::StorageClass;

use super::{pointer::Pointer, Value};
use crate::{
    error::{Error, Result},
    r#type::{ScalarType, Type},
    translation::module::ModuleTranslator,
};
use std::{mem::transmute, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub struct Integer {
    pub source: IntegerSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerKind {
    /// A 32-bit integer
    Short,
    /// A 64-bit integer
    Long,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntegerSource {
    Constant(ConstantSource),
    Conversion(ConversionSource),
    Loaded {
        pointer: Rc<Pointer>,
    },
    FunctionCall {
        args: Box<[Value]>,
        kind: IntegerKind,
    },
    Unary {
        source: UnarySource,
        op1: Rc<Integer>,
    },
    Binary {
        source: BinarySource,
        op1: Rc<Integer>,
        op2: Rc<Integer>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstantSource {
    Short(u32),
    Long(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnarySource {
    Not,
    Negate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinarySource {
    Add,
    Sub,
    Mul,
    SDiv,
    UDiv,
    SRem,
    URem,
    And,
    Or,
    Xor,
    Shl,
    SShr,
    UShr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversionSource {
    FromShort(Rc<Integer>),
    FromLong(Rc<Integer>),
    FromPointer(Rc<Pointer>),
}

impl Integer {
    pub fn new_constant_u32(value: u32) -> Self {
        return Self {
            source: IntegerSource::Constant(ConstantSource::Short(value)),
        };
    }

    pub fn new_constant_i32(value: i32) -> Self {
        return unsafe { Self::new_constant_u32(transmute(value)) };
    }

    pub fn new_constant_u64(value: u64) -> Self {
        return Self {
            source: IntegerSource::Constant(ConstantSource::Long(value)),
        };
    }

    pub fn new_constant_i64(value: i64) -> Self {
        return unsafe { Self::new_constant_u64(transmute(value)) };
    }

    pub fn new_constant_usize(value: u32, module: &ModuleTranslator) -> Self {
        return match module.wasm_memory64 {
            true => Self::new_constant_u64(value as u64),
            false => Self::new_constant_u32(value),
        };
    }

    pub fn new_constant_isize(value: i32, module: &ModuleTranslator) -> Self {
        return match module.wasm_memory64 {
            true => Self::new_constant_i64(value as i64),
            false => Self::new_constant_i32(value),
        };
    }

    pub fn kind(&self, module: &mut ModuleTranslator) -> Result<IntegerKind> {
        return Ok(match &self.source {
            IntegerSource::Loaded { pointer } => match pointer.pointee {
                Type::Scalar(ScalarType::I32) => IntegerKind::Short,
                Type::Scalar(ScalarType::I64) => IntegerKind::Long,
                _ => return Err(Error::unexpected()),
            },
            IntegerSource::FunctionCall { kind, .. } => *kind,
            IntegerSource::Constant(ConstantSource::Long(_)) => IntegerKind::Long,
            IntegerSource::Constant(ConstantSource::Short(_)) => IntegerKind::Short,
            IntegerSource::Conversion(ConversionSource::FromLong(x)) => {
                debug_assert_eq!(x.kind(module)?, IntegerKind::Long);
                IntegerKind::Short
            }
            IntegerSource::Conversion(ConversionSource::FromShort(x)) => {
                debug_assert_eq!(x.kind(module)?, IntegerKind::Short);
                IntegerKind::Long
            }
            IntegerSource::Conversion(ConversionSource::FromPointer(x)) => {
                x.require_addressing(module)?;
                match x.physical_bytes(module) {
                    Some(4) => IntegerKind::Short,
                    Some(8) => IntegerKind::Long,
                    None => return Err(Error::logical_pointer()),
                    _ => return Err(Error::unexpected()),
                }
            }
            IntegerSource::Unary { op1, .. } => op1.kind(module)?,
            IntegerSource::Binary { op1, op2, .. } => {
                let res = op1.kind(module)?;
                debug_assert_eq!(res, op2.kind(module)?);
                res
            }
        });
    }

    pub fn is_isize(
        &self,
        storage_class: StorageClass,
        module: &mut ModuleTranslator,
    ) -> Result<bool> {
        return Ok(self.kind(module)? == IntegerKind::isize(storage_class, module)?);
    }

    pub fn assert_isize(
        &self,
        storage_class: StorageClass,
        module: &mut ModuleTranslator,
    ) -> Result<()> {
        if !self.is_isize(storage_class, module)? {
            return Err(Error::msg(
                "Integer doesn't have the same size as the pointer",
            ));
        }
        return Ok(());
    }

    pub fn to_pointer(
        self: Rc<Self>,
        storage_class: StorageClass,
        pointee: Type,
        module: &mut ModuleTranslator,
    ) -> Result<Pointer> {
        let ptr = Pointer {
            source: super::pointer::PointerSource::FromInteger(self),
            storage_class,
            pointee,
        };

        ptr.require_addressing(module)?;
        return Ok(ptr);
    }

    pub fn negate(self: Rc<Self>) -> Self {
        return Self {
            source: IntegerSource::Unary {
                source: UnarySource::Negate,
                op1: self,
            },
        };
    }

    pub fn add(self: Rc<Self>, rhs: Rc<Integer>, module: &mut ModuleTranslator) -> Result<Self> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        return Ok(Self {
            source: IntegerSource::Binary {
                source: BinarySource::Add,
                op1: self,
                op2: rhs,
            },
        });
    }

    pub fn sub(self: Rc<Self>, rhs: Rc<Integer>, module: &mut ModuleTranslator) -> Result<Self> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        return Ok(Self {
            source: IntegerSource::Binary {
                source: BinarySource::Sub,
                op1: self,
                op2: rhs,
            },
        });
    }
}

impl IntegerKind {
    pub fn isize(storage_class: StorageClass, module: &ModuleTranslator) -> Result<IntegerKind> {
        return match module.spirv_address_bits(storage_class) {
            Some(32) => Ok(IntegerKind::Short),
            Some(64) => Ok(IntegerKind::Long),
            None => Err(Error::logical_pointer()),
            _ => Err(Error::unexpected()),
        };
    }
}

impl From<u32> for Integer {
    fn from(value: u32) -> Self {
        Integer::new_constant_u32(value)
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Integer::new_constant_u64(value)
    }
}

impl From<i32> for Integer {
    fn from(value: i32) -> Self {
        Integer::new_constant_i32(value)
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Integer::new_constant_i64(value)
    }
}
