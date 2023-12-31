#![allow(clippy::should_implement_trait)]

use super::{
    bool::{Bool, BoolSource},
    float::Float,
    pointer::{Pointer, PointerSource},
    vector::Vector,
    Value,
};
use crate::{
    error::{Error, Result},
    fg::module::ModuleBuilder,
    r#type::{PointerSize, ScalarType, Type},
};
use rspirv::spirv::{Capability, StorageClass};
use std::{cell::Cell, mem::transmute, rc::Rc};

#[derive(Debug, Clone)]
pub struct Integer {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub source: IntegerSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerKind {
    /// A 32-bit integer
    Short,
    /// A 64-bit integer
    Long,
}

#[derive(Debug, Clone)]
pub enum IntegerSource {
    FunctionParam(IntegerKind),
    Constant(ConstantSource),
    Conversion(ConversionSource),
    ArrayLength {
        structured_array: Rc<Pointer>,
    },
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
    Select {
        selector: Rc<Bool>,
        true_value: Rc<Integer>,
        false_value: Rc<Integer>,
    },
    Extracted {
        vector: Rc<Vector>,
        index: Rc<Integer>,
    },
    FunctionCall {
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
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
    LeadingZeros,
    TrainlingZeros,
    BitCount,
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
    Rotl,
    Rotr,
}

#[derive(Debug, Clone)]
pub enum ConversionSource {
    Bitcast {
        kind: IntegerKind,
        value: Value,
    },
    FromShort {
        signed: bool,
        value: Rc<Integer>,
    },
    FromLong(Rc<Integer>),
    FromPointer(Rc<Pointer>),
    FromBool(Rc<Bool>, IntegerKind),
    FromFloat {
        kind: IntegerKind,
        signed: bool,
        saturating: bool,
        value: Rc<Float>,
    },
}

impl Integer {
    pub fn new(source: IntegerSource) -> Integer {
        return Self {
            translation: Cell::new(None),
            source,
        };
    }

    pub fn new_constant_u32(value: u32) -> Self {
        return Self {
            translation: Cell::new(None),
            source: IntegerSource::Constant(ConstantSource::Short(value)),
        };
    }

    pub fn new_constant_i32(value: i32) -> Self {
        return unsafe { Self::new_constant_u32(transmute(value)) };
    }

    pub fn new_constant_u64(value: u64) -> Self {
        return Self {
            translation: Cell::new(None),
            source: IntegerSource::Constant(ConstantSource::Long(value)),
        };
    }

    pub fn new_constant_i64(value: i64) -> Self {
        return unsafe { Self::new_constant_u64(transmute(value)) };
    }

    pub fn new_constant_usize(value: u32, module: &ModuleBuilder) -> Self {
        return match module.wasm_memory64 {
            true => Self::new_constant_u64(value as u64),
            false => Self::new_constant_u32(value),
        };
    }

    pub fn new_constant_isize(value: i32, module: &ModuleBuilder) -> Self {
        return match module.wasm_memory64 {
            true => Self::new_constant_i64(value as i64),
            false => Self::new_constant_i32(value),
        };
    }

    pub fn kind(&self, module: &ModuleBuilder) -> Result<IntegerKind> {
        return Ok(match &self.source {
            IntegerSource::Loaded { pointer, .. } => match &pointer.pointee {
                Type::Scalar(ScalarType::I32) => IntegerKind::Short,
                Type::Scalar(ScalarType::I64) => IntegerKind::Long,
                _ => return Err(Error::unexpected()),
            },
            IntegerSource::Select {
                true_value,
                false_value,
                ..
            } => {
                debug_assert_eq!(true_value.kind(module)?, false_value.kind(module)?);
                return true_value.kind(module);
            }
            IntegerSource::Extracted { vector, .. } => match vector.element_type {
                ScalarType::I32 => IntegerKind::Short,
                ScalarType::I64 => IntegerKind::Long,
                _ => return Err(Error::unexpected()),
            },
            IntegerSource::ArrayLength { .. } => IntegerKind::Short,
            IntegerSource::FunctionParam(kind)
            | IntegerSource::FunctionCall { kind, .. }
            | IntegerSource::Conversion(ConversionSource::FromBool(_, kind)) => *kind,
            IntegerSource::Constant(ConstantSource::Long(_)) => IntegerKind::Long,
            IntegerSource::Constant(ConstantSource::Short(_)) => IntegerKind::Short,
            IntegerSource::Conversion(ConversionSource::FromLong(x)) => {
                debug_assert_eq!(x.kind(module)?, IntegerKind::Long);
                IntegerKind::Short
            }
            IntegerSource::Conversion(ConversionSource::FromShort { value, .. }) => {
                debug_assert_eq!(value.kind(module)?, IntegerKind::Short);
                IntegerKind::Long
            }
            IntegerSource::Conversion(
                ConversionSource::FromFloat { kind, .. } | ConversionSource::Bitcast { kind, .. },
            ) => *kind,
            IntegerSource::Conversion(ConversionSource::FromPointer(x)) => {
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

    pub fn get_constant_value(&self) -> Result<Option<ConstantSource>> {
        return Ok(Some(match &self.source {
            IntegerSource::Constant(x) => *x,

            IntegerSource::Conversion(ConversionSource::FromLong(value)) => {
                match value.get_constant_value()? {
                    Some(ConstantSource::Long(x)) => ConstantSource::Short(x as u32),
                    None => return Ok(None),
                    _ => return Err(Error::unexpected()),
                }
            }

            IntegerSource::Conversion(ConversionSource::FromShort {
                signed: true,
                value,
            }) => match value.get_constant_value()? {
                Some(ConstantSource::Short(x)) => unsafe {
                    ConstantSource::Long(transmute(transmute::<_, i32>(x) as i64))
                },
                None => return Ok(None),
                _ => return Err(Error::unexpected()),
            },

            IntegerSource::Conversion(ConversionSource::FromShort {
                signed: false,
                value,
            }) => match value.get_constant_value()? {
                Some(ConstantSource::Short(x)) => ConstantSource::Long(x as u64),
                None => return Ok(None),
                _ => return Err(Error::unexpected()),
            },

            _ => return Ok(None),
        }));
    }

    pub fn is_isize(
        &self,
        storage_class: StorageClass,
        module: &mut ModuleBuilder,
    ) -> Result<bool> {
        return Ok(self.kind(module)? == IntegerKind::isize(storage_class, module)?);
    }

    pub fn assert_isize(
        &self,
        storage_class: StorageClass,
        module: &mut ModuleBuilder,
    ) -> Result<()> {
        if !self.is_isize(storage_class, module)? {
            return Err(Error::msg(
                "Integer doesn't have the same size as the pointer",
            ));
        }
        return Ok(());
    }

    pub fn to_bool(self: Rc<Self>) -> Result<Bool> {
        return Ok(match self.get_constant_value()? {
            Some(ConstantSource::Long(0) | ConstantSource::Short(0)) => {
                Bool::new(BoolSource::Constant(false))
            }
            Some(_) => Bool::new(BoolSource::Constant(true)),
            None => Bool::new(BoolSource::FromInteger(self)),
        });
    }

    pub fn to_pointer(
        self: Rc<Self>,
        size: PointerSize,
        storage_class: StorageClass,
        pointee: Type,
        module: &mut ModuleBuilder,
    ) -> Result<Pointer> {
        match storage_class {
            StorageClass::Generic => module
                .capabilities
                .require_mut(Capability::GenericPointer)?,
            _ => {}
        }

        let ptr = Pointer::new(
            size.to_pointer_kind(),
            storage_class,
            pointee,
            PointerSource::FromInteger(self),
        );

        return Ok(ptr);
    }

    pub fn negate(self: Rc<Self>) -> Self {
        return Self {
            translation: Cell::new(None),
            source: IntegerSource::Unary {
                source: UnarySource::Negate,
                op1: self,
            },
        };
    }

    pub fn add(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x + y))
            }
            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x + y))
            }

            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(rhs),

            _ => IntegerSource::Binary {
                source: BinarySource::Add,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn sub(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Self> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x - y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x - y))
            }

            _ => IntegerSource::Binary {
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

    pub fn mul(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x * y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x * y))
            }

            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(1) | ConstantSource::Long(1))) => return Ok(self),

            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0)))
            | (Some(ConstantSource::Short(1) | ConstantSource::Long(1)), _) => return Ok(rhs),

            _ => IntegerSource::Binary {
                source: BinarySource::Mul,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn s_div(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => {
                return Err(Error::msg("Division by zero"))
            }

            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Short(transmute(
                    transmute::<_, i32>(x) / transmute::<_, i32>(y),
                )))
            },

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Long(transmute(
                    transmute::<_, i64>(x) / transmute::<_, i64>(y),
                )))
            },

            _ => IntegerSource::Binary {
                source: BinarySource::SDiv,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn u_div(
        self: Rc<Self>,
        rhs: Rc<Integer>,
        optimize_away: bool,
        module: &ModuleBuilder,
    ) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => {
                return Err(Error::msg("Division by zero"))
            }

            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x / y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x / y))
            }

            (_, Some(ConstantSource::Short(x))) if x.is_power_of_two() => {
                return self.u_shr(
                    Rc::new(Integer::new_constant_u32(x.ilog2())),
                    optimize_away,
                    module,
                )
            }

            (_, Some(ConstantSource::Long(x))) if x.is_power_of_two() => {
                return self.u_shr(
                    Rc::new(Integer::new_constant_u64(x.ilog2() as u64)),
                    optimize_away,
                    module,
                )
            }

            _ => IntegerSource::Binary {
                source: BinarySource::UDiv,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn s_rem(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => {
                return Err(Error::msg("Division by zero"))
            }

            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Short(transmute(
                    transmute::<_, i32>(x) % transmute::<_, i32>(y),
                )))
            },

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Long(transmute(
                    transmute::<_, i64>(x) % transmute::<_, i64>(y),
                )))
            },

            _ => IntegerSource::Binary {
                source: BinarySource::SRem,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn u_rem(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => {
                return Err(Error::msg("Division by zero"))
            }

            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x % y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x % y))
            }

            _ => IntegerSource::Binary {
                source: BinarySource::URem,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn and(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x & y))
            }
            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x & y))
            }

            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(rhs),

            _ => IntegerSource::Binary {
                source: BinarySource::And,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn or(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x | y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x | y))
            }

            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(rhs),
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(self),

            _ => IntegerSource::Binary {
                source: BinarySource::Or,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn xor(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x ^ y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x ^ y))
            }

            (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _) => return Ok(rhs),

            _ => IntegerSource::Binary {
                source: BinarySource::Xor,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn shl(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x << y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x << y))
            }

            _ => IntegerSource::Binary {
                source: BinarySource::Shl,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn s_shr(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Short(transmute(
                    transmute::<_, i32>(x) >> transmute::<_, i32>(y),
                )))
            },

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => unsafe {
                IntegerSource::Constant(ConstantSource::Long(transmute(
                    transmute::<_, i64>(x) >> transmute::<_, i64>(y),
                )))
            },

            _ => IntegerSource::Binary {
                source: BinarySource::SShr,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn u_shr(
        self: Rc<Self>,
        rhs: Rc<Integer>,
        optimize_away: bool,
        module: &ModuleBuilder,
    ) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(x >> y))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(x >> y))
            }

            (_, Some(x)) if optimize_away => match &self.source {
                IntegerSource::Binary {
                    source: BinarySource::Shl,
                    op1,
                    op2,
                } if op2.get_constant_value()? == Some(x) => return Ok(op1.clone()),
                _ => IntegerSource::Binary {
                    source: BinarySource::UShr,
                    op1: self,
                    op2: rhs,
                },
            },

            _ => IntegerSource::Binary {
                source: BinarySource::UShr,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn clz(self: Rc<Self>) -> Result<Rc<Self>> {
        let source = match self.get_constant_value()? {
            Some(ConstantSource::Short(x)) => {
                IntegerSource::Constant(ConstantSource::Short(u32::leading_zeros(x)))
            }

            Some(ConstantSource::Long(x)) => {
                IntegerSource::Constant(ConstantSource::Long(u64::leading_zeros(x) as u64))
            }

            _ => IntegerSource::Unary {
                source: UnarySource::LeadingZeros,
                op1: self,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn ctz(self: Rc<Self>) -> Result<Rc<Self>> {
        let source = match self.get_constant_value()? {
            Some(ConstantSource::Short(x)) => {
                IntegerSource::Constant(ConstantSource::Short(u32::trailing_zeros(x)))
            }

            Some(ConstantSource::Long(x)) => {
                IntegerSource::Constant(ConstantSource::Long(u64::trailing_zeros(x) as u64))
            }

            _ => IntegerSource::Unary {
                source: UnarySource::TrainlingZeros,
                op1: self,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn popcnt(self: Rc<Self>) -> Result<Rc<Self>> {
        let source = match self.get_constant_value()? {
            Some(ConstantSource::Short(x)) => {
                IntegerSource::Constant(ConstantSource::Short(u32::count_ones(x)))
            }

            Some(ConstantSource::Long(x)) => {
                IntegerSource::Constant(ConstantSource::Long(u64::count_ones(x) as u64))
            }

            _ => IntegerSource::Unary {
                source: UnarySource::BitCount,
                op1: self,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn rotl(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(u32::rotate_left(x, y)))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(u64::rotate_left(x, y as u32)))
            }

            _ => IntegerSource::Binary {
                source: BinarySource::Rotl,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }

    pub fn rotr(self: Rc<Self>, rhs: Rc<Integer>, module: &ModuleBuilder) -> Result<Rc<Self>> {
        match (self.kind(module)?, rhs.kind(module)?) {
            (x, y) if x != y => return Err(Error::mismatch(x, y)),
            _ => {}
        }

        let source = match (self.get_constant_value()?, rhs.get_constant_value()?) {
            (Some(ConstantSource::Short(0) | ConstantSource::Long(0)), _)
            | (_, Some(ConstantSource::Short(0) | ConstantSource::Long(0))) => return Ok(self),

            (Some(ConstantSource::Short(x)), Some(ConstantSource::Short(y))) => {
                IntegerSource::Constant(ConstantSource::Short(u32::rotate_right(x, y)))
            }

            (Some(ConstantSource::Long(x)), Some(ConstantSource::Long(y))) => {
                IntegerSource::Constant(ConstantSource::Long(u64::rotate_right(x, y as u32)))
            }

            _ => IntegerSource::Binary {
                source: BinarySource::Rotr,
                op1: self,
                op2: rhs,
            },
        };

        return Ok(Rc::new(Self {
            translation: Cell::new(None),
            source,
        }));
    }
}

impl IntegerKind {
    pub fn isize(storage_class: StorageClass, module: &ModuleBuilder) -> Result<IntegerKind> {
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
