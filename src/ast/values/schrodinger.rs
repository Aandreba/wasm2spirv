#![allow(clippy::should_implement_trait)]

use super::{integer::Integer, pointer::Pointer, Value};
use crate::{ast::module::ModuleBuilder, error::Result};
use std::{cell::Cell, fmt::Debug, rc::Rc};

/// A value that could be an integer or a pointer, but it's type isn't known until we read it.
pub struct Schrodinger {
    pub source: SchrodingerSource,
    pub(super) integer: Cell<Option<Rc<Integer>>>,
    pub(super) pointer: Cell<Option<Rc<Pointer>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchrodingerSource {
    Loaded {
        pointer: Rc<Pointer>,
    },
    FunctionCall {
        args: Box<[Value]>,
    },
    Add {
        op1: Rc<Schrodinger>,
        op2: Rc<Integer>,
    },
    Sub {
        op1: Rc<Schrodinger>,
        op2: Rc<Integer>,
    },
}

impl Schrodinger {
    pub fn add(self: Rc<Self>, offset: Rc<Integer>) -> Result<Schrodinger> {
        return Ok(Schrodinger {
            source: SchrodingerSource::Add {
                op1: self,
                op2: offset,
            },
            integer: Cell::new(None),
            pointer: Cell::new(None),
        });
    }

    pub fn sub(self: Rc<Self>, offset: Rc<Integer>) -> Result<Schrodinger> {
        return Ok(Schrodinger {
            source: SchrodingerSource::Sub {
                op1: self,
                op2: offset,
            },
            integer: Cell::new(None),
            pointer: Cell::new(None),
        });
    }

    pub fn to_integer(&self, module: &mut ModuleBuilder) -> Result<Rc<Integer>> {
        if let Some(int) = self.integer.take() {
            self.integer.set(Some(int.clone()));
            return Ok(int);
        }

        let int = match &self.source {
            SchrodingerSource::Loaded { pointer } => {
                todo!()
            }
            SchrodingerSource::FunctionCall { args } => {
                todo!()
            }
            SchrodingerSource::Add { op1, op2 } => {
                let op1 = op1.to_integer(module)?;
                op1.add(op2.clone(), module)
            }
            SchrodingerSource::Sub { op1, op2 } => {
                let op1 = op1.to_integer(module)?;
                op1.sub(op2.clone(), module)
            }
        }?;

        let int = Rc::new(int);
        self.integer.set(Some(int.clone()));
        return Ok(int);
    }

    pub fn to_pointer(&self, module: &mut ModuleBuilder) -> Result<Rc<Pointer>> {
        if let Some(ptr) = self.pointer.take() {
            self.pointer.set(Some(ptr.clone()));
            return Ok(ptr);
        }

        let ptr = match &self.source {
            SchrodingerSource::Loaded { pointer } => {
                todo!()
            }
            SchrodingerSource::FunctionCall { args } => {
                todo!()
            }
            SchrodingerSource::Add { op1, op2 } => {
                let op1 = op1.to_pointer(module)?;
                op1.access(op2.clone(), module)
            }
            SchrodingerSource::Sub { op1, op2 } => {
                let op1 = op1.to_pointer(module)?;
                op1.access(op2.clone().negate(), module)
            }
        }?;

        let ptr = Rc::new(ptr);
        self.pointer.set(Some(ptr.clone()));
        return Ok(ptr);
    }
}

impl Debug for Schrodinger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Schrodinger")
            .field("source", &self.source)
            .finish()
    }
}

impl PartialEq for Schrodinger {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}
