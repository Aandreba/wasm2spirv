use super::module::CallableFunction;
use super::values::pointer::Pointer;
use super::{function::FunctionBuilder, module::ModuleBuilder, values::Value, Operation};
use super::{End, Label};
use crate::fg::block::mvp::TranslationResult;
use crate::r#type::PointerSize;
use crate::{
    error::{Error, Result},
    fg::values::{
        float::{Float, FloatKind, FloatSource},
        integer::{Integer, IntegerKind, IntegerSource},
    },
    r#type::{ScalarType, Type},
};
use std::rc::Rc;
use std::{collections::VecDeque, fmt::Debug};
use wasmparser::{BinaryReaderError, Operator, OperatorsReader};

macro_rules! tri {
    ($e:expr) => {
        match $e? {
            x @ (TranslationResult::Found | TranslationResult::Eof) => return Ok(x),
            _ => {}
        }
    };

    (continue $e:expr) => {
        match $e? {
            TranslationResult::Eof => break,
            TranslationResult::Found => continue,
            TranslationResult::NotFound => {}
        }
    };
}

pub mod mvp;

#[derive(Debug, Clone)]
pub enum StackValue {
    Value(Value),
    Schrodinger {
        pointer_variable: Rc<Pointer>,
        loaded_integer: Rc<Integer>,
    },
}

impl<T: Into<Value>> From<T> for StackValue {
    fn from(value: T) -> Self {
        Self::Value(value.into())
    }
}

impl StackValue {
    pub fn to_pointer(
        self,
        size_hint: PointerSize,
        pointee: impl Into<Type>,
        module: &mut ModuleBuilder,
    ) -> Result<Rc<Pointer>> {
        match self {
            StackValue::Value(x) => x.to_pointer(size_hint, pointee, module),
            StackValue::Schrodinger {
                pointer_variable, ..
            } => Ok(pointer_variable.cast(pointee)),
        }
    }

    pub fn convert(self, ty: impl Into<Type>, module: &mut ModuleBuilder) -> Result<Value> {
        let ty = ty.into();
        let instr = match self {
            StackValue::Value(x) => x,
            StackValue::Schrodinger {
                loaded_integer,
                pointer_variable,
            } => {
                if ty == Type::Scalar(module.isize_type()) {
                    return Ok(Value::Integer(loaded_integer));
                }
                Value::Pointer(pointer_variable)
            }
        };

        return Ok(match ty {
            Type::Scalar(ScalarType::I32) => {
                let int = instr.to_integer(IntegerKind::Short, module)?;
                match int.kind(module)? {
                    IntegerKind::Short => int.into(),
                    found => return Err(Error::mismatch(IntegerKind::Short, found)),
                }
            }
            Type::Scalar(ScalarType::I64) => {
                let int = instr.to_integer(IntegerKind::Long, module)?;
                match int.kind(module)? {
                    IntegerKind::Long => int.into(),
                    found => return Err(Error::mismatch(IntegerKind::Long, found)),
                }
            }
            Type::Scalar(ScalarType::Bool) => instr.to_bool(module)?.into(),
            _ => instr,
        });
    }
}

#[derive(Debug, Clone)]
pub struct BlockBuilder<'a> {
    pub reader: BlockReader<'a>,
    pub stack: Vec<StackValue>,
    pub end: End,
    pub outer_labels: VecDeque<Rc<Label>>,
}

pub fn translate_block<'a>(
    reader: BlockReader<'a>,
    labels: VecDeque<Rc<Label>>,
    end: End,
    function: &mut FunctionBuilder,
    module: &mut ModuleBuilder,
) -> Result<BlockBuilder<'a>> {
    let mut result = BlockBuilder {
        stack: Vec::new(),
        reader,
        end,
        outer_labels: labels,
    };

    while let Some(op) = result.reader.next().transpose()? {
        tri!(continue mvp::translate_all(&op, &mut result, function, module));
        return Err(Error::msg(format!("Unknown instruction: {op:?}")));
    }

    return Ok(result);
}

impl<'a> BlockBuilder<'a> {
    pub fn dummy() -> Self {
        return Self {
            reader: BlockReader {
                reader: None,
                cache: VecDeque::new(),
            },
            stack: Vec::new(),
            end: End::Unreachable,
            outer_labels: VecDeque::new(),
        };
    }

    pub fn stack_push(&mut self, value: impl Into<Value>) {
        self.stack.push(StackValue::Value(value.into()));
    }

    pub fn stack_pop_any(&mut self) -> Result<StackValue> {
        if self.stack.is_empty() {
            return Err(Error::msg("Empty stack"));
        }
        Ok(self.stack.remove(self.stack.len() - 1))
    }

    pub fn stack_pop(&mut self, ty: impl Into<Type>, module: &mut ModuleBuilder) -> Result<Value> {
        let ty = ty.into();
        let instr = self.stack_pop_any()?;
        return instr.convert(ty, module);
    }

    pub fn stack_peek(&mut self, ty: impl Into<Type>, module: &mut ModuleBuilder) -> Result<Value> {
        let ty = ty.into();
        let instr = self.stack_peek_any()?;
        return instr.convert(ty, module);
    }

    pub fn stack_peek_any(&mut self) -> Result<StackValue> {
        self.stack
            .last()
            .cloned()
            .ok_or_else(|| Error::msg("Empty stack"))
    }

    pub fn call_function(
        &mut self,
        f: &CallableFunction,
        function: &mut FunctionBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<()> {
        match f {
            CallableFunction::Callback(f) => f(self, function, module),
            CallableFunction::Defined { function_id, ty: f } => {
                let mut args = Vec::with_capacity(f.params().len());
                for ty in f.params().iter().rev() {
                    let raw_arg = self.stack_pop(Type::from(*ty), module)?;
                    args.push(raw_arg);
                }

                args.reverse();
                let args = args.into_boxed_slice();

                assert!(f.results().len() <= 1);
                match f.results().get(0) {
                    Some(wasmparser::ValType::I32) => {
                        self.stack_push(Integer::new(IntegerSource::FunctionCall {
                            function_id: function_id.clone(),
                            args,
                            kind: IntegerKind::Short,
                        }))
                    }
                    Some(wasmparser::ValType::I64) => {
                        self.stack_push(Integer::new(IntegerSource::FunctionCall {
                            function_id: function_id.clone(),
                            args,
                            kind: IntegerKind::Long,
                        }))
                    }
                    Some(wasmparser::ValType::F32) => {
                        self.stack_push(Float::new(FloatSource::FunctionCall {
                            function_id: function_id.clone(),
                            args,
                            kind: FloatKind::Single,
                        }))
                    }
                    Some(wasmparser::ValType::F64) => {
                        self.stack_push(Float::new(FloatSource::FunctionCall {
                            function_id: function_id.clone(),
                            args,
                            kind: FloatKind::Double,
                        }))
                    }
                    None => function.anchors.push(Operation::FunctionCall {
                        function_id: function_id.clone(),
                        args,
                    }),
                    _ => return Err(Error::unexpected()),
                };
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct BlockReader<'a> {
    pub reader: Option<OperatorsReader<'a>>,
    pub cache: VecDeque<Operator<'a>>,
}

impl<'a> BlockReader<'a> {
    pub fn new(reader: OperatorsReader<'a>) -> Self {
        return Self {
            reader: Some(reader),
            cache: VecDeque::new(),
        };
    }

    /// Returns the reader for the current branch
    pub fn split_branch(&mut self) -> Result<BlockReader<'a>, BinaryReaderError> {
        let mut inner_branches = 0u32;

        for (i, op) in self.cache.iter().enumerate() {
            match op {
                Operator::Loop { .. } | Operator::Block { .. } => inner_branches += 1,
                Operator::End => match inner_branches.checked_sub(1) {
                    Some(x) => inner_branches = x,
                    None => {
                        let mut cache = self.cache.split_off(i + 1);
                        core::mem::swap(&mut cache, &mut self.cache);
                        return Ok(BlockReader {
                            reader: None,
                            cache,
                        });
                    }
                },
                _ => continue,
            }
        }

        let mut cache = core::mem::take(&mut self.cache);
        if let Some(ref mut reader) = self.reader {
            loop {
                let op = reader.read()?;
                cache.push_back(op.clone());

                match op {
                    Operator::Loop { .. } | Operator::Block { .. } => inner_branches += 1,
                    Operator::End => match inner_branches.checked_sub(1) {
                        Some(x) => inner_branches = x,
                        None => break,
                    },
                    _ => continue,
                }
            }
        }

        return Ok(BlockReader {
            reader: None,
            cache,
        });
    }
}

impl<'a> Iterator for BlockReader<'a> {
    type Item = Result<Operator<'a>, BinaryReaderError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(op) = self.cache.pop_front() {
            return Some(Ok(op));
        } else if let Some(ref mut reader) = self.reader {
            return reader.read().map(Some).transpose();
        }
        return None;
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.cache.len();
        return (len, self.reader.is_none().then_some(len));
    }
}

impl<'a> Debug for BlockReader<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockReader")
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}
