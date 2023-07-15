use super::values::pointer::Pointer;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FunctionBuilder {
    pub local_variables: Box<[Rc<Pointer>]>,
}

impl FunctionBuilder {
    pub fn new() {}
}
