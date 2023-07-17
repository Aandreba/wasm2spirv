use rspirv::spirv::StorageClass;

use self::{float::Float, integer::Integer, pointer::Pointer, schrodinger::Schrodinger};
use super::module::ModuleBuilder;
use crate::{
    error::{Error, Result},
    r#type::{CompositeType, ScalarType, Type},
    r#type::{ScalarType, Type},
};
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
    pub fn ty(&self, module: &mut ModuleBuilder) -> Result<Type> {
        return Ok(match self {
            Value::Integer(x) => x.kind(module)?.into(),
            Value::Float(x) => x.kind()?.into(),
            Value::Pointer(_) => todo!(),
        });
    }

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

    pub fn to_pointer(
        self,
        pointee: ScalarType,
        byte_offset: impl Into<Rc<Integer>>,
        module: &mut ModuleBuilder,
    ) -> Result<Rc<Pointer>> {
        let pointee = pointee.into();
        match self {
            Value::Integer(x) => {
                return x
                    .to_pointer(StorageClass::Generic, pointee, module)
                    .map(Rc::new)
            }
            Value::Pointer(x) => {
                let ptr = match x.pointee {
                    Type::Composite(CompositeType::StructuredArray(_)) => {
                        let zero = Rc::new(Integer::new_constant_u32(0));
                        x.access_chain([zero.clone(), zero]).map(Rc::new)?
                    }
                    _ => x,
                };
                return Ok(ptr.cast(pointee));
            }
            Value::Schrodinger(x) => return Ok(x.to_pointer(module)?.cast(pointee)),
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
