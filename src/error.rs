use std::error::Error as StdError;
use std::fmt::Display;
use std::{backtrace::Backtrace, borrow::Borrow, fmt::Debug, num::ParseIntError};

use crate::compilers;

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WebAssembly error")]
    Wasm(#[from] wasmparser::BinaryReaderError),

    #[error("WebAssembly text parsing error")]
    Wat(#[from] wat::Error),

    #[error("SPIR-V error")]
    Spirv(#[from] rspirv::dr::Error),

    #[error("Int parsing error")]
    ParseIntError(#[from] ParseIntError),

    #[error("I/O error")]
    Io(#[from] std::io::Error),

    #[error("Compiler error")]
    Compiler(#[from] compilers::CompilerError),

    #[error("Utf-8 parsing error")]
    Utf8(#[from] std::str::Utf8Error),

    #[cfg(feature = "spirv-tools")]
    #[cfg_attr(docsrs, doc(cfg(feature = "spirv-tools")))]
    #[error("Spirv tools error")]
    SpirvTools(#[from] spirv_tools::error::Error),

    #[error("Custom error")]
    Custom(#[from] Box<dyn 'static + Send + Sync + StdError>),
}

impl Error {
    pub fn custom(err: impl 'static + Send + Sync + StdError) -> Self {
        Self::Custom(Box::new(err))
    }

    pub fn msg(msg: impl 'static + Send + Sync + Debug + Display) -> Self {
        #[derive(Debug)]
        #[repr(transparent)]
        struct ErrorMsg<T>(T);

        impl<T: Display> Display for ErrorMsg<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<T: Debug + Display> StdError for ErrorMsg<T> {}

        if cfg!(debug_assertions) {
            println!("{}", Backtrace::capture())
        }

        Self::Custom(Box::new(ErrorMsg(msg)))
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
