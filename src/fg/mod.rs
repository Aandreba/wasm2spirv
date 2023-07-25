use self::{
    function::Storeable,
    values::{bool::Bool, pointer::Pointer, Value},
};
use crate::r#type::Type;
use std::{cell::Cell, rc::Rc};

pub mod block;
pub mod extended_is;
pub mod function;
pub mod import;
pub mod module;
pub mod values;

#[derive(Debug, PartialEq)]
pub enum MergeBlock {
    This,
    Label(Rc<Label>),
}

#[derive(Debug, PartialEq, Default)]
pub struct Label {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
}

#[derive(Debug, Clone)]
pub enum End {
    Return(Option<Type>),
    Unreachable,
}

#[derive(Debug, Clone)]
pub enum Operation {
    Value(Value),
    Label(Rc<Label>),
    Branch {
        label: Rc<Label>,
    },
    BranchConditional {
        condition: Rc<Bool>,
        true_label: Rc<Label>,
        false_label: Rc<Label>,
    },
    Store {
        target: Rc<Pointer>,
        value: Value,
        log2_alignment: Option<u32>,
    },
    Copy {
        src: Rc<Pointer>,
        src_log2_alignment: Option<u32>,
        dst: Rc<Pointer>,
        dst_log2_alignment: Option<u32>,
    },
    FunctionCall {
        function_id: Rc<Cell<Option<rspirv::spirv::Word>>>,
        args: Box<[Value]>,
    },
    Nop,
    Unreachable,
    Return {
        value: Option<Value>,
    },
}

impl Operation {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Operation::Value(x), Operation::Value(y)) => x.ptr_eq(y),
            (Operation::Label(x), Operation::Label(y))
            | (Operation::Branch { label: x }, Operation::Branch { label: y }) => Rc::ptr_eq(x, y),
            (
                Operation::BranchConditional {
                    condition,
                    true_label,
                    false_label,
                },
                Operation::BranchConditional {
                    condition: other_condition,
                    true_label: other_true_label,
                    false_label: other_false_label,
                },
            ) => {
                Rc::ptr_eq(condition, other_condition)
                    && Rc::ptr_eq(true_label, other_true_label)
                    && Rc::ptr_eq(false_label, other_false_label)
            }
            // TODO are ops without values equal?
            _ => false,
        }
    }

    pub fn is_function_terminating(&self) -> bool {
        return matches!(self, Operation::Return { .. } | Operation::Unreachable);
    }

    pub fn is_branch_instruction(&self) -> bool {
        return matches!(
            self,
            Operation::Branch { .. } | Operation::BranchConditional { .. }
        );
    }

    pub fn is_block_terminating(&self) -> bool {
        self.is_function_terminating() || self.is_branch_instruction()
    }
}

impl PartialEq<Rc<Label>> for Operation {
    fn eq(&self, other: &Rc<Label>) -> bool {
        match self {
            Operation::Label(x) => Rc::ptr_eq(x, other),
            _ => false,
        }
    }
}

impl PartialEq<Operation> for Rc<Label> {
    #[inline]
    fn eq(&self, other: &Operation) -> bool {
        other == self
    }
}

impl<T: Into<Value>> From<T> for Operation {
    fn from(value: T) -> Self {
        Operation::Value(value.into())
    }
}
