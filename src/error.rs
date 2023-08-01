use crate::compilers;
use std::error::Error as StdError;
use std::fmt::Display;
use std::{backtrace::Backtrace, borrow::Borrow, fmt::Debug, num::ParseIntError};

pub type Result<T, E = Error> = ::core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("WebAssembly error: {0}")]
    Wasm(#[from] wasmparser::BinaryReaderError),

    #[error("WebAssembly text parsing error: {0}")]
    Wat(#[from] wat::Error),

    #[error("SPIR-V error: {0}")]
    Spirv(#[from] rspirv::dr::Error),

    #[error("Int parsing error: {0}")]
    ParseIntError(#[from] ParseIntError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Compiler error: {0}")]
    Compiler(compilers::CompilerError),

    #[error("Utf-8 parsing error: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[cfg(feature = "tree_sitter")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tree_sitter")))]
    #[error("Tree sitter error: {0}")]
    TreeSitter(#[from] tree_sitter::LanguageError),

    #[cfg(feature = "tree_sitter")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tree_sitter")))]
    #[error("Tree sitter highlighter error: {0}")]
    TreeSitterHighlighter(#[from] tree_sitter_highlight::Error),

    #[cfg(feature = "spirv-tools")]
    #[cfg_attr(docsrs, doc(cfg(feature = "spirv-tools")))]
    #[error("SPIR-V Tools error: {0}")]
    SpirvTools(#[from] spirv_tools::error::Error),

    #[error("Custom error: {0}")]
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
