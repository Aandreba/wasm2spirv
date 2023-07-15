use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Wasm(#[from] wasmparser::BinaryReaderError),
    #[error("{0}")]
    Custom(Cow<'static, str>),
}

impl Error {
    #[inline]
    pub fn msg(msg: impl Into<Cow<'static, str>>) -> Self {
        Self::Custom(msg.into())
    }

    pub fn logical_pointer() -> Self {
        Self::msg("Logical pointers don't have a known physical size")
    }

    pub fn unexpected() -> Self {
        Self::msg("Unexpected error")
    }

    pub fn invalid_operand() -> Self {
        Self::msg("Invalid operand")
    }

    pub fn element_not_found() -> Self {
        Self::msg("Element not found")
    }

    pub fn mismatch(expected: impl Debug, found: impl Debug) -> Self {
        return Self::msg(format!(
            "Mismatched value: expected '{:?}', found '{:?}'",
            expected.borrow(),
            found.borrow()
        ));
    }
}
