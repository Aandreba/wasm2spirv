use super::BlockBuilder;
use crate::{
    error::Result,
    translation::{
        function::FunctionBuilder,
        module::ModuleTranslator,
        values::{float::Float, integer::Integer},
        Operation,
    },
};
use wasmparser::Operator;
use Operator::*;

pub fn translate_constants<'a>(
    op: Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleTranslator,
) -> Result<Option<Operation>> {
    return Ok(Some(match op {
        I32Const { value } => Integer::new_constant_i32(value).into(),
        I64Const { value } => Integer::new_constant_i64(value).into(),
        F32Const { value } => Float::new_constant_f32(f32::from_bits(value.bits())).into(),
        F64Const { value } => Float::new_constant_f64(f64::from_bits(value.bits())).into(),
        _ => return Ok(None),
    }));
}

pub fn translate_control_flow<'a>(
    op: Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleTranslator,
) -> Result<Option<Operation>> {
    return Ok(Some(match op {
        Call { function_index } => {}
        _ => return Ok(None),
    }));
}
