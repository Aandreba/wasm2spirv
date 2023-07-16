use self::{float::Float, integer::Integer, pointer::Pointer, schrodinger::Schrodinger};
use super::module::ModuleBuilder;
use crate::error::{Error, Result};
use std::rc::Rc;

pub mod float;
pub mod integer;
pub mod pointer;
pub mod schrodinger;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(Rc<Integer>),
    Float(Rc<Float>),
    Pointer(Rc<Pointer>),
    Schrodinger(Rc<Schrodinger>),
}

impl Value {
    pub fn i_add(self, rhs: impl Into<Rc<Integer>>, module: &mut ModuleBuilder) -> Result<Value> {
        let rhs = rhs.into();
        return Ok(match self {
            Value::Integer(int) => Value::Integer(Rc::new(int.add(rhs, module)?)),
            Value::Pointer(ptr) => Value::Pointer(Rc::new(ptr.access(rhs, module)?)),
            Value::Schrodinger(sch) => Value::Schrodinger(Rc::new(sch.add(rhs)?)),
            _ => return Err(Error::invalid_operand()),
        });
    }

    pub fn i_sub(self, rhs: impl Into<Rc<Integer>>, module: &mut ModuleBuilder) -> Result<Value> {
        let rhs = rhs.into();
        return Ok(match self {
            Value::Integer(int) => Value::Integer(Rc::new(int.sub(rhs, module)?)),
            Value::Pointer(ptr) => Value::Pointer(Rc::new(ptr.access(rhs.negate(), module)?)),
            Value::Schrodinger(sch) => Value::Schrodinger(Rc::new(sch.sub(rhs)?)),
            _ => return Err(Error::invalid_operand()),
        });
    }

    pub fn to_integer(self, module: &mut ModuleBuilder) -> Result<Rc<Integer>> {
        return match self {
            Value::Integer(x) => Ok(x),
            Value::Pointer(x) => x.to_integer(module).map(Rc::new),
            Value::Schrodinger(x) => x.to_integer(module),
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

impl From<Rc<Schrodinger>> for Value {
    fn from(value: Rc<Schrodinger>) -> Self {
        Value::Schrodinger(value)
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

impl From<Schrodinger> for Value {
    fn from(value: Schrodinger) -> Self {
        Value::Schrodinger(Rc::new(value))
    }
}
