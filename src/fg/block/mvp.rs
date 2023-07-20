use super::{translate_block, BlockBuilder};
use crate::{
    config::MemoryGrowErrorKind,
    error::{Error, Result},
    fg::{
        function::{FunctionBuilder, Storeable},
        module::{GlobalVariable, ModuleBuilder},
        values::{
            bool::{Bool, BoolSource, Comparison},
            float::Float,
            integer::{ConversionSource as IntegerConversionSource, Integer, IntegerSource},
            Value,
        },
        ControlFlow, End, Label, Operation,
    },
    r#type::ScalarType,
};
use std::{cell::Cell, rc::Rc};
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
    tri!(translate_comparison(op, block, module));
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
        Loop { blockty } => {
            let start_label = Rc::new(Label::default());

            function.anchors.push(Operation::Branch {
                label: start_label.clone(),
                control_flow: None,
            });
            function.anchors.push(Operation::Label(start_label.clone()));

            let mut outer_labels = block.outer_labels.clone();
            outer_labels.push_front(start_label);

            let inner_block = block.reader.split_branch()?;
            translate_block(
                inner_block,
                outer_labels,
                End::Unreachable,
                function,
                module,
            )?;
        }

        Block { blockty } => {
            let start_label = Rc::new(Label::default());
            let end_label = Rc::new(Label::default());

            function.anchors.push(Operation::Branch {
                label: start_label.clone(),
                control_flow: None,
            });
            function.anchors.push(Operation::Label(start_label));

            let mut outer_labels = block.outer_labels.clone();
            outer_labels.push_front(end_label.clone());

            let inner_block = block.reader.split_branch()?;
            translate_block(
                inner_block,
                outer_labels,
                End::Unreachable,
                function,
                module,
            )?;

            function.anchors.push(Operation::Label(end_label));
        }

        Br { relative_depth } => {
            let label = block
                .outer_labels
                .get(*relative_depth as usize)
                .ok_or_else(Error::element_not_found)?;

            function.anchors.push(Operation::Branch {
                label: label.clone(),
                control_flow: None,
            })
        }

        BrIf { relative_depth } => {
            let false_label = Rc::new(Label::default());
            let true_label = block
                .outer_labels
                .get(*relative_depth as usize)
                .cloned()
                .ok_or_else(Error::element_not_found)?;

            let condition = block.stack_pop(ScalarType::Bool, module)?.into_bool()?;
            function.anchors.push(Operation::BranchConditional {
                condition,
                true_label: true_label.clone(),
                false_label: false_label.clone(),
                control_flow: Some(ControlFlow::LoopMerge {
                    merge_block: true_label.clone(),
                    continue_target: false_label.clone(),
                }),
            });
            function.anchors.push(Operation::Label(false_label))
        }

        End => {
            let value = match &block.end {
                End::Return(Some(ty)) => Some(block.stack_pop(ty.clone(), module)?),
                _ => None,
            };

            function.anchors.push(Operation::End {
                kind: block.end.clone(),
                value,
            });

            return Ok(TranslationResult::Eof);
        }

        Call { function_index } => {
            let f = module
                .functions
                .get(*function_index as usize)
                .cloned()
                .ok_or_else(Error::element_not_found)?;

            block.call_function(&f, function, module)?;
        }

        Select => {
            let _false_operand = block.stack_pop_any()?;
            let _true_operand = block.stack_pop_any()?;
            let _selector = block.stack_pop(ScalarType::Bool, module)?.into_bool()?;
            todo!()
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
                } => {
                    let value = pointer.clone().load(None, block, module)?;
                    block.stack_push(value)
                }

                Storeable::Schrodinger(sch) => {
                    let value = sch.load(block, module)?;
                    block.stack_push(value)
                }
            }
        }

        LocalSet { local_index } => local_set(*local_index, false, block, function, module)?,
        LocalTee { local_index } => local_set(*local_index, true, block, function, module)?,

        GlobalGet { global_index } => {
            let var = module
                .global_variables
                .get(*global_index as usize)
                .ok_or_else(Error::element_not_found)?;

            let var = match var {
                GlobalVariable::Variable(var) => var.clone().load(None, block, module)?,
                GlobalVariable::Constant(c) => c.clone(),
            };
            block.stack_push(var);
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
                    var.store(value, None, block, module)?
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
            let value = pointer.load(Some(memarg.align as u32), block, module)?;
            block.stack_push(value);
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
            function.anchors.push(pointer.store(
                value,
                Some(memarg.align as u32),
                block,
                module,
            )?);
        }

        I32Load8U { memarg } | I64Load8U { memarg } => {
            let (ty, mask) = match op {
                I32Load8U { .. } => (ScalarType::I32, Integer::new_constant_u32(0xff)),
                I32Load8U { .. } => (ScalarType::I64, Integer::new_constant_u64(0xff)),
                _ => return Err(Error::unexpected()),
            };

            let value = block.stack_pop(ty, module)?.into_integer()?;
            value.and(Rc::new(mask), module)?.into()
        }

        MemorySize { .. } => match module.memory_grow_error {
            MemoryGrowErrorKind::Hard => return Err(Error::msg("Spirv cannot allocate memory")),
            MemoryGrowErrorKind::Soft => block.stack_push(Integer::new_constant_isize(-1, module)),
        },

        MemoryGrow { .. } => {
            return Err(Error::msg(
                "Spirv doesn't have heap memory. Memory size cannot be grown",
            ))
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
        I32WrapI64 => {
            let operand = block.stack_pop(ScalarType::I64, module)?.into_integer()?;
            Integer::new(IntegerSource::Conversion(
                IntegerConversionSource::FromLong(operand),
            ))
            .into()
        }

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
            op1.i_add(op2, module)?
        }

        I32Sub | I64Sub => {
            let ty: ScalarType = match op {
                I32Sub => ScalarType::I32,
                I64Sub => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop_any()?;
            op1.i_sub(op2, module)?
        }

        I32Mul | I64Mul => {
            let ty: ScalarType = match op {
                I32Mul => ScalarType::I32,
                I64Mul => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.mul(op2, module)?.into()
        }

        I32DivS | I64DivS => {
            let ty: ScalarType = match op {
                I32DivS => ScalarType::I32,
                I64DivS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.s_div(op2, module)?.into()
        }

        I32DivU | I64DivU => {
            let ty: ScalarType = match op {
                I32DivU => ScalarType::I32,
                I64DivU => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.u_div(op2, module)?.into()
        }

        F32Add | F64Add => {
            let ty: ScalarType = match op {
                F32Add => ScalarType::F32,
                F64Add => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.add(op2)?.into()
        }

        F32Sub | F64Sub => {
            let ty: ScalarType = match op {
                F32Sub => ScalarType::F32,
                F64Sub => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.sub(op2)?.into()
        }

        F32Mul | F64Mul => {
            let ty: ScalarType = match op {
                F32Mul => ScalarType::F32,
                F64Mul => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.mul(op2)?.into()
        }

        F32Div | F64Div => {
            let ty: ScalarType = match op {
                F32Div => ScalarType::F32,
                F64Div => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.div(op2)?.into()
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
        I32Shl | I64Shl => {
            let ty = match op {
                I32Shl => ScalarType::I32,
                I64Shl => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
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

pub fn translate_comparison<'a>(
    op: &Operator<'a>,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<TranslationResult> {
    let instr: Value = match op {
        I32GeU | I64GeU | I32GeS | I64GeS => {
            let ty = match op {
                I32GeU | I32GeS => ScalarType::I32,
                I64GeU | I64GeS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntComparison {
                kind: Comparison::Ge,
                signed: matches!(op, I32GeS | I64GeS),
                op1,
                op2,
            })
            .into()
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
                .push(pointer.clone().store(value, None, block, module)?);
        }

        Storeable::Schrodinger(sch) => {
            let value = match peek {
                true => block.stack_peek_any()?,
                false => block.stack_pop_any()?,
            };

            let (op1, op2) = match value {
                Value::Integer(int) => sch.store_integer(int, block, module),
                Value::Pointer(ptr) => sch.store_pointer(ptr, block, module),
                _ => return Err(Error::unexpected()),
            }?;

            function.anchors.push(op1);
            function.anchors.extend(op2);
        }
    };

    return Ok(());
}
