use super::{function::FunctionBuilder, module::ModuleBuilder, values::Value, Operation};
use crate::ast::block::mvp::TranslationResult;
use crate::{
    ast::values::{
        float::{Float, FloatKind, FloatSource},
        integer::{Integer, IntegerKind, IntegerSource},
    },
    error::{Error, Result},
    r#type::{ScalarType, Type},
};
use std::cell::Cell;
use std::{collections::VecDeque, fmt::Debug};
use wasmparser::{BinaryReaderError, FuncType, Operator, OperatorsReader};

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
pub struct BlockBuilder<'a> {
    pub reader: BlockReader<'a>,
    /// Instructions who's order **must** be followed
    pub anchors: Vec<Operation>,
    pub stack: VecDeque<Value>,
    pub return_ty: Option<Type>,
}

impl<'a> BlockBuilder<'a> {
    pub fn new(
        reader: BlockReader<'a>,
        return_ty: impl Into<Option<Type>>,
        function: &mut FunctionBuilder,
        module: &mut ModuleBuilder,
    ) -> Result<Self> {
        let mut result = Self {
            anchors: Vec::new(),
            stack: VecDeque::new(),
            reader,
            return_ty: return_ty.into(),
        };

        while let Some(op) = result.reader.next().transpose()? {
            tri!(continue mvp::translate_all(&op, &mut result, function, module));
            return Err(Error::msg(format!("Unknown instruction: {op:?}")));
        }

        return Ok(result);
    }

    pub fn stack_push(&mut self, value: impl Into<Value>) {
        self.stack.push_back(value.into());
    }

    pub fn stack_pop_any(&mut self) -> Result<Value> {
        self.stack
            .pop_back()
            .ok_or_else(|| Error::msg("Empty stack"))
    }

    pub fn stack_pop(&mut self, ty: impl Into<Type>, module: &mut ModuleBuilder) -> Result<Value> {
        let ty = ty.into();
        let instr = self.stack_pop_any()?;

        return Ok(match ty {
            Type::Scalar(ScalarType::I32) if !module.wasm_memory64 => {
                let int = instr.to_integer(module)?;
                match int.kind(module)? {
                    IntegerKind::Short => int.into(),
                    found => return Err(Error::mismatch(IntegerKind::Short, found)),
                }
            }
            Type::Scalar(ScalarType::I64) if module.wasm_memory64 => {
                let int = instr.to_integer(module)?;
                match int.kind(module)? {
                    IntegerKind::Long => int.into(),
                    found => return Err(Error::mismatch(IntegerKind::Long, found)),
                }
            }
            _ => {
                todo!()
            }
        });
    }

    pub fn stack_peek(&mut self) -> Result<Value> {
        self.stack
            .back()
            .cloned()
            .ok_or_else(|| Error::msg("Empty stack"))
    }

    pub fn call_function(&mut self, f: &FuncType, module: &mut ModuleBuilder) -> Result<Operation> {
        let mut args = Vec::with_capacity(f.params().len());
        for ty in f.params().iter().rev() {
            let raw_arg = self.stack_pop(Type::from(*ty), module)?;
            args.push(raw_arg);
        }

        args.reverse();
        let args = args.into_boxed_slice();

        assert!(f.results().len() <= 1);
        return Ok(match f.results().get(0) {
            Some(wasmparser::ValType::I32) => Integer {
                translation: Cell::new(None),
                source: IntegerSource::FunctionCall {
                    args,
                    kind: IntegerKind::Short,
                },
            }
            .into(),
            Some(wasmparser::ValType::I64) => Integer {
                translation: Cell::new(None),
                source: IntegerSource::FunctionCall {
                    args,
                    kind: IntegerKind::Long,
                },
            }
            .into(),
            Some(wasmparser::ValType::F32) => Float {
                translation: Cell::new(None),
                source: FloatSource::FunctionCall {
                    args,
                    kind: FloatKind::Single,
                },
            }
            .into(),
            Some(wasmparser::ValType::F64) => Float {
                translation: Cell::new(None),
                source: FloatSource::FunctionCall {
                    args,
                    kind: FloatKind::Double,
                },
            }
            .into(),
            None => Operation::FunctionCall { args },
            _ => return Err(Error::unexpected()),
        });
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
                        let mut cache = self.cache.split_off(i);
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
