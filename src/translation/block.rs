use super::{function::FunctionBuilder, values::Value, Operation};
use crate::error::Result;
use std::collections::VecDeque;
use wasmparser::{BinaryReaderError, Operator, OperatorsReader};

pub mod mvp;

pub struct BlockBuilder<'a> {
    pub reader: BlockReader<'a>,
    /// Instructions who's order **must** be followed
    pub anchors: Vec<Operation>,
    pub stack: VecDeque<Value>,
}

impl<'a> BlockBuilder<'a> {
    pub fn new(mut reader: BlockReader<'a>, function: &mut FunctionBuilder) -> Result<Self> {
        let mut result = Self {
            anchors: Vec::new(),
            stack: VecDeque::new(),
            reader,
        };

        while let Some(op) = result.reader.next().transpose()? {}

        return Ok(result);
    }
}

pub struct BlockReader<'a> {
    pub reader: Option<OperatorsReader<'a>>,
    pub cache: VecDeque<Operator<'a>>,
}

impl<'a> BlockReader<'a> {
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
