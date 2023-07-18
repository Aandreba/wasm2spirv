use super::BlockBuilder;
use crate::{
    ast::{
        function::{FunctionBuilder, Storeable},
        module::{GlobalVariable, ModuleBuilder},
        values::{
            float::Float,
            integer::{ConversionSource as IntegerConversionSource, Integer, IntegerSource},
            Value,
        },
        Operation,
    },
    error::{Error, Result},
    r#type::ScalarType,
};
use std::cell::Cell;
use wasmparser::Operator;
use Operator::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationResult {
    Found,
    NotFound,
    Eof,
}

pub fn translate_all<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    tri!(translate_constants(op, block));
    tri!(translate_control_flow(op, block, function, module));
    tri!(translate_conversion(op, block, module));
    tri!(translate_variables(op, block, function, module));
    tri!(translate_memory(op, block, function, module));
    tri!(translate_arith(op, block, module));
    tri!(translate_logic(op, block, module));
    return Ok(TranslationResult::NotFound);
}

pub fn translate_constants<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
) -> Result<TranslationResult> {
    let instr: Value = match op {
        I32Const { value } => Integer::new_constant_i32(*value).into(),
        I64Const { value } => Integer::new_constant_i64(*value).into(),
        F32Const { value } => Float::new_constant_f32(f32::from_bits(value.bits())).into(),
        F64Const { value } => Float::new_constant_f64(f64::from_bits(value.bits())).into(),
        _ => return Ok(TranslationResult::NotFound),
    };

    block.stack_push(instr);
    return Ok(TranslationResult::Found);
}

pub fn translate_control_flow<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    match op {
        End => {
            let return_value = block
                .return_ty
                .clone()
                .map(|ty| block.stack_pop(ty, module))
                .transpose()?;
            function.anchors.push(Operation::End { return_value });
            return Ok(TranslationResult::Eof);
        }

        Call { function_index } => {
            let f = module
                .functions
                .get(*function_index as usize)
                .ok_or_else(Error::element_not_found)?;

            block.call_function(&f, block, function, module)?;
        }
        _ => return Ok(TranslationResult::NotFound),
    }

    return Ok(TranslationResult::Found);
}

pub fn translate_variables<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    match op {
        LocalGet { local_index } => {
            let var = function
                .local_variables
                .get(*local_index as usize)
                .ok_or_else(Error::element_not_found)?;

            match var {
                Storeable::Pointer {
                    pointer,
                    is_extern_pointer: true,
                } => block.stack_push(pointer.clone()),

                Storeable::Pointer {
                    pointer,
                    is_extern_pointer: false,
                } => block.stack_push(pointer.clone().load(None, module)?),

                Storeable::Schrodinger(sch) => block.stack_push(sch.load(module)?),
            }
        }

        LocalSet { local_index } => local_set(*local_index, false, block, function, module)?,
        LocalTee { local_index } => local_set(*local_index, true, block, function, module)?,

        GlobalGet { global_index } => {
            let var = module
                .global_variables
                .get(*global_index as usize)
                .ok_or_else(Error::element_not_found)?;

            block.stack_push(match var {
                GlobalVariable::Variable(var) => var.clone().load(None, module)?,
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
                    var.store(value, None, module)?
                }
                GlobalVariable::Constant(_) => {
                    return Err(Error::msg("Tried to update a constant global variable"))
                }
            };

            function.anchors.push(op);
        }

        Drop => {
            let _ = block.stack_pop_any()?;
        }

        _ => return Ok(TranslationResult::NotFound),
    }

    return Ok(TranslationResult::Found);
}

pub fn translate_memory<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    match op {
        I32Load { memarg } | F32Load { memarg } | I64Load { memarg } | F64Load { memarg } => {
            let pointee = match op {
                I32Load { .. } => ScalarType::I32,
                F32Load { .. } => ScalarType::F32,
                I64Load { .. } => ScalarType::I64,
                F64Load { .. } => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let offset = Integer::new_constant_usize(memarg.offset as u32, module);
            let pointer = block.stack_pop_any()?.to_pointer(pointee, offset, module)?;
            block.stack_push(pointer.load(Some(memarg.align as u32), module)?);
        }

        I32Store { memarg } | F32Store { memarg } | I64Store { memarg } | F64Store { memarg } => {
            let pointee = match op {
                I32Store { .. } => ScalarType::I32,
                F32Store { .. } => ScalarType::F32,
                I64Store { .. } => ScalarType::I64,
                F64Store { .. } => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let value = block.stack_pop(pointee, module)?;
            let offset = Integer::new_constant_usize(memarg.offset as u32, module);
            let pointer = block.stack_pop_any()?.to_pointer(pointee, offset, module)?;
            function
                .anchors
                .push(pointer.store(value, Some(memarg.align as u32), module)?);
        }
        _ => return Ok(TranslationResult::NotFound),
    }

    return Ok(TranslationResult::Found);
}

pub fn translate_conversion<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    let instr: Value = match op {
        I64ExtendI32U | I64ExtendI32S => Integer {
            translation: Cell::new(None),
            source: IntegerSource::Conversion(IntegerConversionSource::FromShort {
                signed: matches!(op, I64ExtendI32S),
                value: match block.stack_pop(ScalarType::I32, module)? {
                    Value::Integer(int) => int,
                    _ => return Err(Error::unexpected()),
                },
            }),
        }
        .into(),
        _ => return Ok(TranslationResult::NotFound),
    };

    block.stack_push(instr);
    return Ok(TranslationResult::Found);
}

pub fn translate_arith<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    let instr: Value = match op {
        I32Add | I64Add => {
            let op2 = block.stack_pop_any()?;
            let op1 = block.stack_pop_any()?;
            op1.add(op2, module)?
        }
        _ => return Ok(TranslationResult::NotFound),
    };

    block.stack_push(instr);
    return Ok(TranslationResult::Found);
}

pub fn translate_logic<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    let instr: Value = match op {
        I32Shl => {
            let op2 = block.stack_pop(ScalarType::I32, module)?;
            let op1 = block.stack_pop(ScalarType::I32, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.shl(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }
        _ => return Ok(TranslationResult::NotFound),
    };

    block.stack_push(instr);
    return Ok(TranslationResult::Found);
}

fn local_set<'a>(
    local_index: u32,
    peek: bool,
    block: &mut BlockBuilder<'a>,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<()> {
    let var = function
        .local_variables
        .get(local_index as usize)
        .ok_or_else(Error::element_not_found)?;

    match var {
        Storeable::Pointer { pointer, .. } => {
            let value = match peek {
                true => block.stack_peek(pointer.element_type(), module)?,
                false => block.stack_pop(pointer.element_type(), module)?,
            };

            function
                .anchors
                .push(pointer.clone().store(value, None, module)?);
        }

        Storeable::Schrodinger(sch) => {
            let value = match peek {
                true => block.stack_peek_any()?,
                false => block.stack_pop_any()?,
            };

            let op = match value {
                Value::Integer(int) => sch.store_integer(int, module),
                Value::Pointer(ptr) => sch.store_pointer(ptr, module),
                _ => return Err(Error::unexpected()),
            }?;

            function.anchors.push(op);
        }
    };

    return Ok(());
}
