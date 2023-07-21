#![allow(clippy::should_implement_trait)]

use super::{
    float::{Float, FloatSource},
    integer::{Integer, IntegerSource},
    pointer::Pointer,
    Value,
};
use crate::r#type::{CompositeType, ScalarType};
use std::{cell::Cell, rc::Rc};

#[derive(Debug, Clone)]
pub struct Vector {
    pub(crate) translation: Cell<Option<rspirv::spirv::Word>>,
    pub source: VectorSource,
    pub element_type: ScalarType,
    pub element_count: u32,
}

#[derive(Debug, Clone)]
pub enum VectorSource {
    Loaded {
        pointer: Rc<Pointer>,
        log2_alignment: Option<u32>,
    },
}

impl Vector {
    pub fn new(source: VectorSource, element_type: ScalarType, element_count: u32) -> Self {
        return Self {
            translation: Cell::new(None),
            source,
            element_type,
            element_count,
        };
    }

    pub fn vector_type(&self) -> CompositeType {
        CompositeType::Vector(self.element_type, self.element_count)
    }

    pub fn extract(self: Rc<Self>, index: impl Into<Rc<Integer>>) -> Value {
        match self.element_type {
            ScalarType::I32 | ScalarType::I64 => Integer::new(IntegerSource::Extracted {
                vector: self,
                index: index.into(),
            })
            .into(),
            ScalarType::F32 | ScalarType::F64 => Float::new(FloatSource::Extracted {
                vector: self,
                index: index.into(),
            })
            .into(),
            _ => todo!(),
        }
    }
}
