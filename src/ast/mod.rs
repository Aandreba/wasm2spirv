use self::{
    function::Storeable,
    values::{bool::Bool, Value},
};
use crate::r#type::Type;
use std::{cell::Cell, rc::Rc};

pub mod block;
pub mod function;
pub mod import;
pub mod module;
pub mod values;

#[derive(Debug, Clone, PartialEq)]
pub enum ControlFlow {
    LoopMerge {
        merge_block: Rc<Label>,
        continue_target: Rc<Label>,
    },
    SelectionMerge(Rc<Label>),
}

#[derive(Debug, PartialEq)]
pub struct Label {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum End {
    Return(Option<Type>),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operation {
    Value(Value),
    Label(Rc<Label>),
    Branch {
        label: Rc<Label>,
        control_flow: ControlFlow,
    },
    BranchConditional {
        condition: Rc<Bool>,
        true_label: Rc<Label>,
        false_label: Rc<Label>,
        control_flow: ControlFlow,
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
        kind: End,
        value: Option<Value>,
    },
}

impl<T: Into<Value>> From<T> for Operation {
    fn from(value: T) -> Self {
        Operation::Value(value.into())
    }
}
