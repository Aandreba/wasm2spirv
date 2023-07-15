use super::values::pointer::Pointer;
use std::rc::Rc;
use wasmparser::FuncType;

pub struct FunctionBuilder {
    pub local_variables: Box<[Rc<Pointer>]>,
}
