use std::{borrow::Cow, fmt::Debug};

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
}
