use self::{function::Storeable, values::Value};
use std::{cell::Cell, rc::Rc};

pub mod block;
pub mod function;
pub mod import;
pub mod module;
pub mod values;

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Value(Value),
    Store {
        target: Storeable,
        value: Value,
        log2_alignment: Option<u32>,
    },
    FunctionCall {
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
        args: Box<[Value]>,
    },
    Nop,
    Unreachable,
    End {
        return_value: Option<Value>,
    },
}

impl<T: Into<Value>> From<T> for Operation {
    fn from(value: T) -> Self {
        Operation::Value(value.into())
    }
}
