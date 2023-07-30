use crate::translation::Translation;
use serde::{Deserialize, Serialize};
use std::{cell::Cell, fmt::Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ExtendedSet {
    // https://registry.khronos.org/SPIR-V/specs/unified1/GLSL.std.450.html
    GLSL450,
    // https://registry.khronos.org/SPIR-V/specs/unified1/OpenCL.ExtendedInstructionSet.100.html
    OpenCL,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum GLSLInstr {
    Sqrt = 31,
    Fabs = 4,
    Ceil = 9,
    Floor = 8,
    Trunc = 3,
    RoundEven = 2,
    Fmin = 37,
    Fmax = 40,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum OpenCLInstr {
    Sqrt = 61,
    Clz = 151,
    Ctz = 152,
    Popcount = 166,
    // Left rotation
    Rotate = 161,
    Fabs = 23,
    Ceil = 12,
    Floor = 25,
    Trunc = 66,
    Rint = 53,
    Copysign = 13,
    Fmin = 28,
    Fmax = 27,
}

impl Display for ExtendedSet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtendedSet::GLSL450 => write!(f, "GLSL.std.450"),
            ExtendedSet::OpenCL => write!(f, "OpenCL.std"),
        }
    }
}

#[derive(Debug)]
pub struct ExtendedIs {
    pub(crate) translation: Cell<Option<spirv::Word>>,
    pub kind: ExtendedSet,
}

impl ExtendedIs {
    pub fn new(kind: ExtendedSet) -> Self {
        return Self {
            translation: Cell::new(None),
            kind,
        };
    }
}

impl Translation for &ExtendedIs {
    fn translate(
        self,
        _: &super::module::ModuleBuilder,
        _: Option<&super::function::FunctionBuilder>,
        builder: &mut crate::translation::Builder,
    ) -> crate::error::Result<rspirv::spirv::Word> {
        if let Some(word) = self.translation.get() {
            return Ok(word);
        }

        let word = builder.ext_inst_import(self.kind.to_string());
        self.translation.set(Some(word));
        return Ok(word);
    }
}
