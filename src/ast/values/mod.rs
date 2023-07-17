use self::{float::Float, integer::Integer, pointer::Pointer};
use super::module::ModuleBuilder;
use crate::{
    error::{Error, Result},
    r#type::{ScalarType, Type},
};
use rspirv::spirv::StorageClass;
use std::rc::Rc;

pub mod float;
pub mod integer;
pub mod pointer;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(Rc<Integer>),
    Float(Rc<Float>),
    Pointer(Rc<Pointer>),
}

impl Value {
    pub fn ty(&self, module: &mut ModuleBuilder) -> Result<Type> {
        return Ok(match self {
            Value::Integer(x) => x.kind(module)?.into(),
            Value::Float(x) => x.kind()?.into(),
            Value::Pointer(_) => todo!(),
        });
    }

    pub fn function_parameter(ty: impl Into<Type>) -> Result<Value> {
        match ty.into() {
            Type::Scalar(ty) => {}
            _ => todo!(),
        }
    }

    pub fn add(self, rhs: impl Into<Value>, module: &mut ModuleBuilder) -> Result<Value> {
        return match (self, rhs.into()) {
            (Value::Integer(x), Value::Integer(y)) => x.add(y, module).map(Into::into),
            (Value::Pointer(x), Value::Integer(y)) => x.access(y, module).map(Into::into),
            (Value::Integer(x), Value::Pointer(y)) => y.access(x, module).map(Into::into),
            (Value::Float(x), Value::Float(y)) => todo!(),
            (x, y) => return Err(Error::msg(format!("Invalid operands:\n\t{x:?}\n\t{y:?}"))),
        };
    }

    pub fn i_sub(self, rhs: impl Into<Rc<Integer>>, module: &mut ModuleBuilder) -> Result<Value> {
        let rhs = rhs.into();
        return Ok(match self {
            Value::Integer(int) => Value::Integer(Rc::new(int.sub(rhs, module)?)),
            Value::Pointer(ptr) => Value::Pointer(Rc::new(ptr.access(rhs.negate(), module)?)),
            _ => return Err(Error::invalid_operand()),
        });
    }

    pub fn to_integer(self, module: &mut ModuleBuilder) -> Result<Rc<Integer>> {
        return match self {
            Value::Integer(x) => Ok(x),
            Value::Pointer(x) => x.to_integer(module).map(Rc::new),
            _ => return Err(Error::invalid_operand()),
        };
    }

    pub fn to_pointer(
        self,
        pointee: ScalarType,
        byte_offset: impl Into<Rc<Integer>>,
        module: &mut ModuleBuilder,
    ) -> Result<Rc<Pointer>> {
        let pointee = pointee.into();
        return match self {
            Value::Integer(x) => x
                .to_pointer(StorageClass::Generic, pointee, module)
                .map(Rc::new)?
                .access(byte_offset, module)
                .map(Rc::new),
            Value::Pointer(x) => Ok(Rc::new(x.access(byte_offset, module)?).cast(pointee)),
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
