use super::BlockBuilder;
use crate::{
    error::{Error, Result},
    r#type::ScalarType,
    translation::{
        function::FunctionBuilder,
        module::{GlobalVariable, ModuleBuilder},
        values::{
            float::Float,
            integer::{ConversionSource as IntegerConversionSource, Integer, IntegerSource},
            Value,
        },
        Operation,
    },
};
use wasmparser::Operator;
use Operator::*;

pub fn translate_all<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<bool> {
    tri!(translate_constants(op, block));
    tri!(translate_control_flow(op, block, module));
    tri!(translate_conversion(op, block, module));
    tri!(translate_variables(op, block, function, module));
    return Ok(false);
}

pub fn translate_constants<'a>(op: &Operator<'a>, block: &mut BlockBuilder<'a>) -> Result<bool> {
    let instr: Value = match op {
        I32Const { value } => Integer::new_constant_i32(*value).into(),
        I64Const { value } => Integer::new_constant_i64(*value).into(),
        F32Const { value } => Float::new_constant_f32(f32::from_bits(value.bits())).into(),
        F64Const { value } => Float::new_constant_f64(f64::from_bits(value.bits())).into(),
        _ => return Ok(false),
    };

    block.stack_push(instr);
    return Ok(true);
}

pub fn translate_control_flow<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<bool> {
    match op {
        Call { function_index } => {
            let function = module
                .functions
                .get(*function_index as usize)
                .cloned()
                .ok_or_else(Error::element_not_found)?;

            match block.call_function(&function, module)? {
                Operation::Value(res) => block.stack_push(res),
                op @ Operation::FunctionCall { .. } => block.anchors.push(op),
                _ => return Err(Error::unexpected()),
            }
        }
        _ => return Ok(false),
    }

    return Ok(true);
}

pub fn translate_variables<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<bool> {
    match op {
        LocalGet { local_index } => {
            let var = function
                .local_variables
                .get(*local_index as usize)
                .ok_or_else(Error::element_not_found)?;
            block.stack_push(var.clone().load(module)?);
        }

        LocalSet { local_index } => {
            let var = function
                .local_variables
                .get(*local_index as usize)
                .ok_or_else(Error::element_not_found)?;

            let value = block.stack_pop(var.element_type(), module)?;
            block.anchors.push(var.clone().store(value, module)?);
        }

        LocalTee { local_index } => {
            let var = function
                .local_variables
                .get(*local_index as usize)
                .ok_or_else(Error::element_not_found)?;

            let value = block.stack_peek()?;
            block.anchors.push(var.clone().store(value, module)?);
        }

        GlobalGet { global_index } => {
            let var = module
                .global_variables
                .get(*global_index as usize)
                .ok_or_else(Error::element_not_found)?;

            block.stack_push(match var {
                GlobalVariable::Variable(var) => var.clone().load(module)?,
                GlobalVariable::Constant(c) => c.clone(),
            });
        }

        GlobalSet { global_index } => {
            let var = module
                .global_variables
                .get(*global_index as usize)
                .cloned()
                .ok_or_else(Error::element_not_found)?;

            let op = match var {
                GlobalVariable::Variable(var) => {
                    let value = block.stack_pop(var.element_type(), module)?;
                    var.store(value, module)?
                }
                GlobalVariable::Constant(_) => {
                    return Err(Error::msg("Tried to update a constant global variable"))
                }
            };

            block.anchors.push(op);
        }

        _ => return Ok(false),
    }

    return Ok(true);
}

pub fn translate_conversion<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<bool> {
    let instr: Value = match op {
        I64ExtendI32U | I64ExtendI32S => Integer {
            source: IntegerSource::Conversion(IntegerConversionSource::FromShort {
                signed: matches!(op, I64ExtendI32S),
                value: match block.stack_pop(ScalarType::I32, module)? {
                    Value::Integer(int) => int,
                    _ => return Err(Error::unexpected()),
                },
            }),
        }
        .into(),
        _ => return Ok(false),
    };

    return Ok(true);
}
