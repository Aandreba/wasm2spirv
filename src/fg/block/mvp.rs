use super::{translate_block, BlockBuilder};
use crate::{
    config::MemoryGrowErrorKind,
    error::{Error, Result},
    fg::{
        function::{FunctionBuilder, Storeable},
        module::{GlobalVariable, ModuleBuilder},
        values::{
            bool::{Bool, BoolSource, Comparison, Equality},
            float::{Float, FloatSource},
            integer::{
                ConversionSource as IntegerConversionSource, Integer, IntegerKind, IntegerSource,
            },
            pointer::{Pointer, PointerSource},
            Value,
        },
        End, Label, Operation,
    },
    r#type::{PointerSize, ScalarType},
};
use std::{cell::Cell, rc::Rc};
use tracing::debug;
use wasmparser::{MemArg, Operator};
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

            if !function
                .anchors
                .last()
                .is_some_and(Operation::is_block_terminating)
            {
                function.anchors.push(Operation::Branch {
                    label: end_label.clone(),
                });
            } else {
                debug!("{:?}", function.anchors.last());
            }

            function.anchors.push(Operation::Label(end_label));
        }

        Br { relative_depth } => {
            let label = block
                .outer_labels
                .get(*relative_depth as usize)
                .ok_or_else(Error::element_not_found)?;

            function.anchors.push(Operation::Branch {
                label: label.clone(),
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
            });
            function.anchors.push(Operation::Label(false_label))
        }

        End | Return => {
            let value = match &block.end {
                End::Return(Some(ty)) => Some(block.stack_pop(ty.clone(), module)?),
                End::Return(None) => None,
                _ => return Ok(TranslationResult::Eof),
            };

            function.anchors.push(Operation::Return { value });
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
            let selector = block.stack_pop(ScalarType::Bool, module)?.into_bool()?;
            let false_operand = block.stack_pop_any()?;
            let true_operand = block.stack_pop_any()?;

            let value = match selector.get_constant_value()? {
                Some(true) => true_operand,
                Some(false) => false_operand,
                _ => match (true_operand, false_operand) {
                    (Value::Bool(true_value), false_value) => Bool::new(BoolSource::Select {
                        selector,
                        true_value,
                        false_value: false_value.to_bool(module)?,
                    })
                    .into(),

                    (Value::Integer(true_value), false_value) => {
                        let kind = true_value.kind(module)?;
                        Integer::new(IntegerSource::Select {
                            selector,
                            true_value,
                            false_value: false_value.to_integer(kind, module)?,
                        })
                        .into()
                    }

                    (Value::Pointer(true_value), false_value) => {
                        let pointee = true_value.pointee.clone();
                        let storage_class = true_value.storage_class;
                        let size = true_value.kind.to_pointer_size();

                        Pointer::new(
                            size.to_pointer_kind(),
                            storage_class,
                            pointee,
                            PointerSource::Select {
                                selector,
                                false_value: false_value.to_pointer(
                                    size,
                                    pointee.clone(),
                                    module,
                                )?,
                                true_value,
                            },
                        )
                        .into()
                    }

                    (Value::Float(true_value), Value::Float(false_value)) => {
                        Float::new(FloatSource::Select {
                            selector,
                            true_value,
                            false_value,
                        })
                        .into()
                    }

                    (Value::Vector(true_value), Value::Vector(false_value)) => {
                        todo!()
                    }

                    _ => return Err(Error::unexpected()),
                },
            };

            block.stack_push(value)
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
                Storeable::Pointer(pointer) => block.stack_push(pointer.clone()),
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
                    let value = block.stack_pop(var.pointee, module)?;
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
            let pointer = block
                .stack_pop_any()?
                .to_pointer(PointerSize::Skinny, pointee, module)?
                .access(offset, module)
                .map(Rc::new)?;

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
            let pointer = block
                .stack_pop_any()?
                .to_pointer(PointerSize::Skinny, pointee, module)?
                .access(offset, module)
                .map(Rc::new)?;

            function.anchors.push(pointer.store(
                value,
                Some(memarg.align as u32),
                block,
                module,
            )?);
        }

        I32Load8U { memarg } => load_byte(IntegerKind::Short, memarg, block, module)?,
        I64Load8U { memarg } => load_byte(IntegerKind::Long, memarg, block, module)?,

        I32Load16U { memarg } => {
            todo!()
        }

        MemorySize { .. } => {
            let zero = Integer::new_constant_usize(0, module);
            block.stack_push(zero)
        }

        MemoryGrow { .. } => match module.memory_grow_error {
            MemoryGrowErrorKind::Hard => return Err(Error::msg("SPIR-V cannot allocate memory")),
            MemoryGrowErrorKind::Soft => block.stack_push(Integer::new_constant_isize(-1, module)),
        },

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
                value: block.stack_pop(ScalarType::I32, module)?.into_integer()?,
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

        I32RemS | I64RemS => {
            let ty: ScalarType = match op {
                I32RemS => ScalarType::I32,
                I64RemS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.s_rem(op2, module)?.into()
        }

        I32RemU | I64RemU => {
            let ty: ScalarType = match op {
                I32RemU => ScalarType::I32,
                I64RemU => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.u_rem(op2, module)?.into()
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
        I32And | I64And => {
            let ty = match op {
                I32And => ScalarType::I32,
                I64And => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.and(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }

        I32Or | I64Or => {
            let ty = match op {
                I32Or => ScalarType::I32,
                I64Or => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.or(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }

        I32Xor | I64Xor => {
            let ty = match op {
                I32Xor => ScalarType::I32,
                I64Xor => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.xor(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }

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

        I32ShrS | I64ShrS => {
            let ty = match op {
                I32ShrS => ScalarType::I32,
                I64ShrS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.s_shr(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }

        I32ShrU | I64ShrU => {
            let ty = match op {
                I32ShrU => ScalarType::I32,
                I64ShrU => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?;
            let op1 = block.stack_pop(ty, module)?;
            match (op1, op2) {
                (Value::Integer(x), Value::Integer(y)) => x.u_shr(y, module)?.into(),
                _ => return Err(Error::unexpected()),
            }
        }

        I32Clz | I64Clz => {
            let ty = match op {
                I32Clz => ScalarType::I32,
                I64Clz => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.clz()?.into()
        }

        I32Ctz | I64Ctz => {
            let ty = match op {
                I32Ctz => ScalarType::I32,
                I64Ctz => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.ctz()?.into()
        }

        I32Popcnt | I64Popcnt => {
            let ty = match op {
                I32Popcnt => ScalarType::I32,
                I64Popcnt => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.popcnt()?.into()
        }

        I32Rotl | I64Rotl => {
            let ty = match op {
                I32Rotl => ScalarType::I32,
                I64Rotl => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.rotl(op2, module)?.into()
        }

        I32Rotr | I64Rotr => {
            let ty = match op {
                I32Rotr => ScalarType::I32,
                I64Rotr => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            op1.rotr(op2, module)?.into()
        }

        F32Abs | F64Abs => {
            let ty = match op {
                F32Abs => ScalarType::F32,
                F64Abs => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.abs()?.into()
        }

        F32Neg | F64Neg => {
            let ty = match op {
                F32Neg => ScalarType::F32,
                F64Neg => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.neg()?.into()
        }

        F32Ceil | F64Ceil => {
            let ty = match op {
                F32Ceil => ScalarType::F32,
                F64Ceil => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.ceil()?.into()
        }

        F32Floor | F64Floor => {
            let ty = match op {
                F32Floor => ScalarType::F32,
                F64Floor => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.floor()?.into()
        }

        F32Trunc | F64Trunc => {
            let ty = match op {
                F32Trunc => ScalarType::F32,
                F64Trunc => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.trunc()?.into()
        }

        F32Nearest | F64Nearest => {
            let ty = match op {
                F32Nearest => ScalarType::F32,
                F64Nearest => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.nearest()?.into()
        }

        F32Min | F64Min => {
            let ty: ScalarType = match op {
                F32Min => ScalarType::F32,
                F64Min => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.min(op2)?.into()
        }

        F32Max | F64Max => {
            let ty: ScalarType = match op {
                F32Max => ScalarType::F32,
                F64Max => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            op1.max(op2)?.into()
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

        I32GtU | I64GtU | I32GtS | I64GtS => {
            let ty = match op {
                I32GtU | I32GtS => ScalarType::I32,
                I64GtU | I64GtS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntComparison {
                kind: Comparison::Gt,
                signed: matches!(op, I32GtS | I64GtS),
                op1,
                op2,
            })
            .into()
        }

        I32Eq | I64Eq => {
            let ty = match op {
                I32Eq => ScalarType::I32,
                I64Eq => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntEquality {
                kind: Equality::Eq,
                op1,
                op2,
            })
            .into()
        }

        I32Eqz | I64Eqz => {
            let (ty, op2) = match op {
                I32Eqz => (ScalarType::I32, Rc::new(Integer::new_constant_i32(0))),
                I64Eqz => (ScalarType::I64, Rc::new(Integer::new_constant_i64(0))),
                _ => return Err(Error::unexpected()),
            };

            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntEquality {
                kind: Equality::Eq,
                op1,
                op2,
            })
            .into()
        }

        I32Ne | I64Ne => {
            let ty = match op {
                I32Ne => ScalarType::I32,
                I64Ne => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntEquality {
                kind: Equality::Ne,
                op1,
                op2,
            })
            .into()
        }

        I32LtU | I64LtU | I32LtS | I64LtS => {
            let ty = match op {
                I32LtU | I32LtS => ScalarType::I32,
                I64LtU | I64LtS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntComparison {
                kind: Comparison::Lt,
                signed: matches!(op, I32LtS | I64LtS),
                op1,
                op2,
            })
            .into()
        }

        I32LeU | I64LeU | I32LeS | I64LeS => {
            let ty = match op {
                I32LeU | I32LeS => ScalarType::I32,
                I64LeU | I64LeS => ScalarType::I64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_integer()?;
            let op1 = block.stack_pop(ty, module)?.into_integer()?;
            Bool::new(BoolSource::IntComparison {
                kind: Comparison::Le,
                signed: matches!(op, I32LeS | I64LeS),
                op1,
                op2,
            })
            .into()
        }

        F32Le | F64Le => {
            let ty = match op {
                F32Le => ScalarType::F32,
                F64Le => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatComparison {
                kind: Comparison::Le,
                op1,
                op2,
            })
            .into()
        }

        F32Lt | F64Lt => {
            let ty = match op {
                F32Lt => ScalarType::F32,
                F64Lt => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatComparison {
                kind: Comparison::Lt,
                op1,
                op2,
            })
            .into()
        }

        F32Eq | F64Eq => {
            let ty = match op {
                F32Eq => ScalarType::F32,
                F64Eq => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatEquality {
                kind: Equality::Eq,
                op1,
                op2,
            })
            .into()
        }

        F32Ne | F64Ne => {
            let ty = match op {
                F32Ne => ScalarType::F32,
                F64Ne => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatEquality {
                kind: Equality::Ne,
                op1,
                op2,
            })
            .into()
        }

        F32Gt | F64Gt => {
            let ty = match op {
                F32Gt => ScalarType::F32,
                F64Gt => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatComparison {
                kind: Comparison::Gt,
                op1,
                op2,
            })
            .into()
        }

        F32Ge | F64Ge => {
            let ty = match op {
                F32Ge => ScalarType::F32,
                F64Ge => ScalarType::F64,
                _ => return Err(Error::unexpected()),
            };

            let op2 = block.stack_pop(ty, module)?.into_float()?;
            let op1 = block.stack_pop(ty, module)?.into_float()?;
            Bool::new(BoolSource::FloatComparison {
                kind: Comparison::Ge,
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

fn load_byte<'a>(
    kind: IntegerKind,
    memarg: &MemArg,
    block: &mut BlockBuilder<'a>,
    module: &mut ModuleBuilder,
) -> Result<()> {
    let zero = Rc::new(Integer::new_constant_usize(0, module));
    let eight = Rc::new(Integer::new_constant_usize(8, module));

    let (shift_offset, stride, mask) = match kind {
        IntegerKind::Short => (
            Rc::new(Integer::new_constant_usize(3, &module)),
            Rc::new(Integer::new_constant_u32(4)),
            Rc::new(Integer::new_constant_u32(0xff)),
        ),
        IntegerKind::Long => (
            Rc::new(Integer::new_constant_usize(7, &module)),
            Rc::new(Integer::new_constant_u64(8)),
            Rc::new(Integer::new_constant_u64(0xff)),
        ),
    };

    // Take pointer by parts
    let pointer = block
        .stack_pop_any()?
        .to_pointer(PointerSize::Skinny, kind, module)?;
    let byte_offset = pointer.byte_offset();

    // Calculate true offset
    let constant_offset = Rc::new(Integer::new_constant_usize(memarg.offset as u32, module));
    let byte_offset = match byte_offset {
        Some(byte_offset) => byte_offset.add(constant_offset, module)?,
        None => constant_offset,
    };

    // Get value of unadapted integer
    let value = pointer
        .access(byte_offset.clone(), module)
        .map(Rc::new)?
        .load(Some(memarg.align as u32), block, module)?
        .into_integer()?;

    let shift = shift_offset
        .sub(byte_offset.u_rem(stride, module)?, module)
        .map(Rc::new)?
        .mul(eight, module)?;

    let result = value.u_shr(shift, module)?.and(mask, module)?;
    block.stack_push(result);
    Ok(())
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
        Storeable::Pointer(pointer) => {
            let value = match peek {
                true => block.stack_peek(pointer.pointee.clone(), module)?,
                false => block.stack_pop(pointer.pointee.clone(), module)?,
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
