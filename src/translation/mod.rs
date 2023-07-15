use rspirv::spirv::Op;
use std::rc::Rc;

pub mod block;
pub mod function;
pub mod module;

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    Operation { opcode: Op },
    Load { pointer: Rc<Instruction> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Instruction(Rc<Instruction>),
    LiteralU32(u32),
    LiteralU64(u64),
    LiteralF32(f32),
    LiteralF64(f64),
}
