#![allow(clippy::should_implement_trait)]

use super::pointer::Pointer;
use crate::{
    error::{Error, Result},
    r#type::{ScalarType, Type},
    translation::module::ModuleTranslator,
};
use rspirv::spirv::StorageClass;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatKind {
    /// A 32-bit integer
    Single,
    /// A 64-bit integer
    Double,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Float {
    Constant(ConstantSource),
    Conversion(ConversionSource),
    Loaded {
        pointer: Rc<Pointer>,
    },
    Unary {
        source: UnarySource,
        op1: Rc<Float>,
    },
    Binary {
        source: BinarySource,
        op1: Rc<Float>,
        op2: Rc<Float>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstantSource {
    Single(f32),
    Double(f64),
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
    Div,
    Sqrt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConversionSource {
    FromSingle(Rc<Float>),
    FromDouble(Rc<Float>),
}

impl Float {
    pub fn new_constant_f32(value: f32) -> Self {
        return Float::Constant(ConstantSource::Single(value));
    }

    pub fn new_constant_f64(value: f64) -> Self {
        return Float::Constant(ConstantSource::Double(value));
    }

    pub fn kind(&self) -> Result<FloatKind> {
        return Ok(match self {
            Float::Loaded { pointer } => match pointer.pointee {
                Type::Scalar(ScalarType::F32) => FloatKind::Single,
                Type::Scalar(ScalarType::F64) => FloatKind::Double,
                _ => return Err(Error::unexpected()),
            },
            Float::Constant(ConstantSource::Double(_)) => FloatKind::Double,
            Float::Constant(ConstantSource::Single(_)) => FloatKind::Single,
            Float::Conversion(ConversionSource::FromDouble(x)) => {
                debug_assert_eq!(x.kind()?, FloatKind::Double);
                FloatKind::Single
            }
            Float::Conversion(ConversionSource::FromSingle(x)) => {
                debug_assert_eq!(x.kind()?, FloatKind::Single);
                FloatKind::Double
            }
            Float::Unary { op1, .. } => op1.kind()?,
            Float::Binary { op1, op2, .. } => {
                let res = op1.kind()?;
                debug_assert_eq!(res, op2.kind()?);
                res
            }
        });
    }

    pub fn negate(self: Rc<Self>) -> Self {
        return Float::Unary {
            source: UnarySource::Negate,
            op1: self,
        };
    }

    pub fn add(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        return Ok(Float::Binary {
            source: BinarySource::Add,
            op1: self,
            op2: rhs,
        });
    }

    pub fn sub(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        return Ok(Float::Binary {
            source: BinarySource::Sub,
            op1: self,
            op2: rhs,
        });
    }
}

impl FloatKind {
    pub fn isize(storage_class: StorageClass, module: &ModuleTranslator) -> Result<FloatKind> {
        return match module.spirv_address_bits(storage_class) {
            Some(32) => Ok(FloatKind::Single),
            Some(64) => Ok(FloatKind::Double),
            None => Err(Error::logical_pointer()),
            _ => Err(Error::unexpected()),
        };
    }
}

impl From<f32> for Float {
    fn from(value: f32) -> Self {
        Float::new_constant_f32(value)
    }
}

impl From<f64> for Float {
    fn from(value: f64) -> Self {
        Float::new_constant_f64(value)
    }
}
