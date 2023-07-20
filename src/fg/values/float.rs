#![allow(clippy::should_implement_trait)]

use super::{integer::Integer, pointer::Pointer, vector::Vector, Value};
use crate::{
    error::{Error, Result},
    r#type::{ScalarType, Type},
};
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub struct Float {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub source: FloatSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FloatKind {
    /// A 32-bit integer
    Single,
    /// A 64-bit integer
    Double,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FloatSource {
    FunctionParam(FloatKind),
    Constant(ConstantSource),
    Conversion(ConversionSource),
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
    Extracted {
        vector: Rc<Vector>,
        index: Rc<Integer>,
    },
    FunctionCall {
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
        args: Box<[Value]>,
        kind: FloatKind,
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
    pub fn new(source: FloatSource) -> Float {
        return Self {
            translation: Cell::new(None),
            source,
        };
    }

    pub fn new_constant_f32(value: f32) -> Self {
        return Self {
            translation: Cell::new(None),
            source: FloatSource::Constant(ConstantSource::Single(value)),
        };
    }

    pub fn new_constant_f64(value: f64) -> Self {
        return Self {
            translation: Cell::new(None),
            source: FloatSource::Constant(ConstantSource::Double(value)),
        };
    }

    pub fn kind(&self) -> Result<FloatKind> {
        return Ok(match &self.source {
            FloatSource::Loaded { pointer, .. } => match pointer.element_type() {
                Type::Scalar(ScalarType::F32) => FloatKind::Single,
                Type::Scalar(ScalarType::F64) => FloatKind::Double,
                _ => return Err(Error::unexpected()),
            },
            FloatSource::Extracted { vector, .. } => match vector.element_type {
                ScalarType::F32 => FloatKind::Single,
                ScalarType::F64 => FloatKind::Double,
                _ => return Err(Error::unexpected()),
            },
            FloatSource::FunctionParam(kind) | FloatSource::FunctionCall { kind, .. } => *kind,
            FloatSource::Constant(ConstantSource::Double(_)) => FloatKind::Double,
            FloatSource::Constant(ConstantSource::Single(_)) => FloatKind::Single,
            FloatSource::Conversion(ConversionSource::FromDouble(x)) => {
                debug_assert_eq!(x.kind()?, FloatKind::Double);
                FloatKind::Single
            }
            FloatSource::Conversion(ConversionSource::FromSingle(x)) => {
                debug_assert_eq!(x.kind()?, FloatKind::Single);
                FloatKind::Double
            }
            FloatSource::Unary { op1, .. } => op1.kind()?,
            FloatSource::Binary { op1, op2, .. } => {
                let res = op1.kind()?;
                debug_assert_eq!(res, op2.kind()?);
                res
            }
        });
    }

    pub fn get_constant_value(&self) -> Result<Option<ConstantSource>> {
        return Ok(Some(match &self.source {
            FloatSource::Constant(x) => *x,
            FloatSource::Conversion(ConversionSource::FromDouble(x)) => {
                match x.get_constant_value()? {
                    Some(ConstantSource::Double(x)) => ConstantSource::Single(x as f32),
                    _ => return Err(Error::unexpected()),
                }
            }
            FloatSource::Conversion(ConversionSource::FromSingle(x)) => {
                match x.get_constant_value()? {
                    Some(ConstantSource::Single(x)) => ConstantSource::Double(x as f64),
                    _ => return Err(Error::unexpected()),
                }
            }
            _ => return Ok(None),
        }));
    }

    pub fn negate(self: Rc<Self>) -> Self {
        return Self {
            translation: Cell::new(None),
            source: FloatSource::Unary {
                source: UnarySource::Negate,
                op1: self,
            },
        };
    }

    pub fn add(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Single(x)), Some(ConstantSource::Single(y))) => {
                FloatSource::Constant(ConstantSource::Single(x + y))
            }
            (Some(ConstantSource::Double(x)), Some(ConstantSource::Double(y))) => {
                FloatSource::Constant(ConstantSource::Double(x + y))
            }
            _ => FloatSource::Binary {
                source: BinarySource::Add,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Self {
            translation: Cell::new(None),
            source,
        });
    }

    pub fn sub(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Single(x)), Some(ConstantSource::Single(y))) => {
                FloatSource::Constant(ConstantSource::Single(x - y))
            }
            (Some(ConstantSource::Double(x)), Some(ConstantSource::Double(y))) => {
                FloatSource::Constant(ConstantSource::Double(x - y))
            }
            _ => FloatSource::Binary {
                source: BinarySource::Sub,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Self {
            translation: Cell::new(None),
            source,
        });
    }

    pub fn mul(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Single(x)), Some(ConstantSource::Single(y))) => {
                FloatSource::Constant(ConstantSource::Single(x * y))
            }
            (Some(ConstantSource::Double(x)), Some(ConstantSource::Double(y))) => {
                FloatSource::Constant(ConstantSource::Double(x * y))
            }
            _ => FloatSource::Binary {
                source: BinarySource::Mul,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Self {
            translation: Cell::new(None),
            source,
        });
    }

    pub fn div(self: Rc<Self>, rhs: Rc<Float>) -> Result<Self> {
        match (self.kind()?, rhs.kind()?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Single(x)), Some(ConstantSource::Single(y))) => {
                FloatSource::Constant(ConstantSource::Single(x / y))
            }
            (Some(ConstantSource::Double(x)), Some(ConstantSource::Double(y))) => {
                FloatSource::Constant(ConstantSource::Double(x / y))
            }
            _ => FloatSource::Binary {
                source: BinarySource::Div,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Self {
            translation: Cell::new(None),
            source,
        });
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
