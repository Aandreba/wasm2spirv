#![allow(clippy::should_implement_trait)]

use super::{
    integer::{ConstantSource, Integer},
    pointer::Pointer,
};
use crate::error::Result;
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone, PartialEq)]
pub struct Bool {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub source: BoolSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoolSource {
    Constant(bool),
    FromInteger(Rc<Integer>),
    Negated(Rc<Bool>),
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
}

impl Bool {
    pub fn new(source: BoolSource) -> Self {
        return Self {
            translation: Cell::new(None),
            source,
        };
    }

    pub fn get_constant_value(&self) -> Result<Option<bool>> {
        match &self.source {
            BoolSource::Constant(x) => Ok(Some(*x)),
            BoolSource::FromInteger(x) => match x.get_constant_value()? {
                Some(ConstantSource::Long(0) | ConstantSource::Short(0)) => Ok(Some(false)),
                Some(_) => Ok(Some(true)),
                None => Ok(None),
            },
            BoolSource::Negated(x) => Ok(x.get_constant_value()?.map(|x| !x)),
            _ => return Ok(None),
        }
    }
}
