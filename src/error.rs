use std::fmt::Debug;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Wasm(#[from] wasmparser::BinaryReaderError),
}
