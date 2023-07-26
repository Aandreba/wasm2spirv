use self::{
    bool::Bool,
    float::{Float, FloatKind, FloatSource},
    integer::{Integer, IntegerKind, IntegerSource},
    pointer::{Pointer, PointerSource},
    vector::Vector,
};
use super::module::ModuleBuilder;
use crate::{
    error::{Error, Result},
    r#type::{CompositeType, PointerSize, ScalarType, Type},
};
use rspirv::spirv::StorageClass;
use std::rc::Rc;

pub mod bool;
pub mod float;
pub mod integer;
pub mod pointer;
pub mod vector;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(Rc<Integer>),
    Float(Rc<Float>),
    Pointer(Rc<Pointer>),
    Vector(Rc<Vector>),
    Bool(Rc<Bool>),
}

impl Value {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(x), Value::Integer(y)) => Rc::ptr_eq(x, y),
            (Value::Float(x), Value::Float(y)) => Rc::ptr_eq(x, y),
            (Value::Pointer(x), Value::Pointer(y)) => Rc::ptr_eq(x, y),
            (Value::Vector(x), Value::Vector(y)) => Rc::ptr_eq(x, y),
            (Value::Bool(x), Value::Bool(y)) => Rc::ptr_eq(x, y),
            _ => false,
        }
    }

    pub fn ty(&self, module: &ModuleBuilder) -> Result<Type> {
        return Ok(match self {
            Value::Bool(_) => Type::Scalar(ScalarType::Bool),
            Value::Integer(x) => x.kind(module)?.into(),
            Value::Float(x) => x.kind()?.into(),
            Value::Pointer(x) => {
                Type::pointer(x.kind.to_pointer_size(), x.storage_class, x.pointee.clone())
            }
            Value::Vector(x) => {
                Type::Composite(CompositeType::Vector(x.element_type, x.element_count))
            }
        });
    }

    pub fn function_parameter(ty: impl Into<Type>) -> Value {
        match ty.into() {
            Type::Scalar(ScalarType::I32) => {
                Integer::new(IntegerSource::FunctionParam(integer::IntegerKind::Short)).into()
            }
            Type::Scalar(ScalarType::I64) => {
                Integer::new(IntegerSource::FunctionParam(integer::IntegerKind::Long)).into()
            }
            Type::Scalar(ScalarType::F32) => {
                Float::new(FloatSource::FunctionParam(FloatKind::Single)).into()
            }
            Type::Scalar(ScalarType::F64) => {
                Float::new(FloatSource::FunctionParam(FloatKind::Double)).into()
            }
            Type::Pointer {
                size,
                storage_class,
                pointee,
            } => Pointer::new(
                size.to_pointer_kind(),
                storage_class,
                *pointee,
                PointerSource::FunctionParam,
            )
            .into(),
            _ => todo!(),
        }
    }

    pub fn i_add(self, rhs: impl Into<Value>, module: &mut ModuleBuilder) -> Result<Value> {
        return match (self, rhs.into()) {
            (Value::Integer(x), Value::Integer(y)) => x.add(y, module).map(Into::into),
            (Value::Pointer(x), Value::Integer(y)) => x.access(y, module).map(Into::into),
            (Value::Integer(x), Value::Pointer(y)) => y.access(x, module).map(Into::into),
            (x, y) => return Err(Error::msg(format!("Invalid operands:\n\t{x:?}\n\t{y:?}"))),
        };
    }

    pub fn i_sub(self, rhs: impl Into<Rc<Integer>>, module: &mut ModuleBuilder) -> Result<Value> {
        let rhs = rhs.into();
        return Ok(match self {
            Value::Integer(int) => Value::Integer(Rc::new(int.sub(rhs, module)?)),
            Value::Pointer(ptr) => {
                Value::Pointer(Rc::new(ptr.access(Rc::new(rhs.negate()), module)?))
            }
            _ => return Err(Error::invalid_operand()),
        });
    }

    pub fn into_integer(self) -> Result<Rc<Integer>> {
        match self {
            Value::Integer(x) => Ok(x),
            other => Err(Error::msg(format!("Expected an integer, found {other:?}"))),
        }
    }

    pub fn into_float(self) -> Result<Rc<Float>> {
        match self {
            Value::Float(x) => Ok(x),
            other => Err(Error::msg(format!("Expected a float, found {other:?}"))),
        }
    }

    pub fn into_pointer(self) -> Result<Rc<Pointer>> {
        match self {
            Value::Pointer(x) => Ok(x),
            other => Err(Error::msg(format!("Expected a pointer, found {other:?}"))),
        }
    }

    pub fn into_vector(self) -> Result<Rc<Vector>> {
        match self {
            Value::Vector(x) => Ok(x),
            other => Err(Error::msg(format!("Expected a vector, found {other:?}"))),
        }
    }

    pub fn into_bool(self) -> Result<Rc<Bool>> {
        match self {
            Value::Bool(x) => Ok(x),
            other => Err(Error::msg(format!("Expected a boolean, found {other:?}"))),
        }
    }

    pub fn to_bool(self, module: &mut ModuleBuilder) -> Result<Rc<Bool>> {
        return match self {
            Value::Bool(x) => Ok(x),
            Value::Integer(x) => x.to_bool().map(Rc::new),
            Value::Pointer(x) => x.to_integer(module).map(Rc::new)?.to_bool().map(Rc::new),
            _ => return Err(Error::invalid_operand()),
        };
    }

    pub fn to_integer(self, kind: IntegerKind, module: &mut ModuleBuilder) -> Result<Rc<Integer>> {
        return match self {
            Value::Integer(x) if kind == x.kind(module)? => Ok(x),
            Value::Pointer(x) if kind == module.isize_integer_kind() => {
                x.to_integer(module).map(Rc::new)
            }
            Value::Bool(x) => x.to_integer(kind),
            _ => return Err(Error::invalid_operand()),
        };
    }

    pub fn to_pointer(
        self,
        size_hint: PointerSize,
        pointee: impl Into<Type>,
        module: &mut ModuleBuilder,
    ) -> Result<Rc<Pointer>> {
        let pointee = pointee.into();
        return match self {
            Value::Integer(x) => x
                .to_pointer(size_hint, StorageClass::Generic, pointee.into(), module)
                .map(Rc::new),
            Value::Pointer(x) => Ok(x.cast(pointee)),
            _ => return Err(Error::invalid_operand()),
        };
    }
}

impl From<Rc<Integer>> for Value {
    fn from(value: Rc<Integer>) -> Self {
        Value::Integer(value)
    }
}

impl From<Rc<Float>> for Value {
    fn from(value: Rc<Float>) -> Self {
        Value::Float(value)
    }
}

impl From<Rc<Pointer>> for Value {
    fn from(value: Rc<Pointer>) -> Self {
        Value::Pointer(value)
    }
}

impl From<Rc<Vector>> for Value {
    fn from(value: Rc<Vector>) -> Self {
        Value::Vector(value)
    }
}

impl From<Rc<Bool>> for Value {
    fn from(value: Rc<Bool>) -> Self {
        Value::Bool(value)
    }
}

impl From<Integer> for Value {
    fn from(value: Integer) -> Self {
        Value::Integer(Rc::new(value))
    }
}

impl From<Float> for Value {
    fn from(value: Float) -> Self {
        Value::Float(Rc::new(value))
    }
}

impl From<Pointer> for Value {
    fn from(value: Pointer) -> Self {
        Value::Pointer(Rc::new(value))
    }
}

impl From<Vector> for Value {
    fn from(value: Vector) -> Self {
        Value::Vector(Rc::new(value))
    }
}

impl From<Bool> for Value {
    fn from(value: Bool) -> Self {
        Value::Bool(Rc::new(value))
    }
}
