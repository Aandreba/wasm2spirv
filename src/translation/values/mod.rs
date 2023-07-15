use self::{integer::Integer, pointer::Pointer};
use std::rc::Rc;

pub mod integer;
pub mod pointer;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(Rc<Integer>),
    Pointer(Rc<Pointer>),
}
