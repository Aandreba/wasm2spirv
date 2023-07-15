use super::values::pointer::Pointer;
use std::rc::Rc;

pub struct FunctionBuilder {
    pub local_variables: Box<[Rc<Pointer>]>,
}

pub enum CallableFunction {
    Imported,
    Declared,
}
