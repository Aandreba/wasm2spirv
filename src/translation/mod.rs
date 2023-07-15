use self::values::{pointer::Pointer, Value};
use std::rc::Rc;

pub mod block;
pub mod function;
pub mod module;
pub mod values;

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Value(Value),
    Store { pointer: Rc<Pointer>, value: Value },
    FunctionCall { args: Box<[Value]> },
    Nop,
    Unreachable,
}

impl<T: Into<Value>> From<T> for Operation {
    fn from(value: T) -> Self {
        Operation::Value(value.into())
    }
}
