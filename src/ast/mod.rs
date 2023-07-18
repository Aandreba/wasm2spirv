use self::{
    function::Storeable,
    values::{bool::Bool, Value},
};
use std::{cell::Cell, rc::Rc};

pub mod block;
pub mod function;
pub mod import;
pub mod module;
pub mod values;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Label(pub(crate) Cell<Option<rspirv::spirv::Word>>);

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Value(Value),
    Label(Rc<Label>),
    Branch(Rc<Label>),
    BranchConditional {
        condition: Rc<Bool>,
        true_label: Rc<Label>,
        false_label: Rc<Label>,
    },
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
