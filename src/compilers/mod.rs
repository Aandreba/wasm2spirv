use std::sync::Arc;

use docfg::docfg;

#[cfg(feature = "naga")]
pub mod naga;

// #[cfg(feature = "spirv_cross")]
// pub mod spvc;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CompilerError {
    #[cfg(feature = "spirv_cross")]
    #[cfg_attr(docsrs, doc(cfg(feature = "spirv_cross")))]
    #[error("Spirv cross error")]
    SpirvCross(#[from] spirv_cross::ErrorCode),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga error")]
    NagaValidation(#[from] ::naga::WithSpan<::naga::valid::ValidationError>),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga SPIR-V error\n{0}")]
    NagaSpv(Arc<::naga::front::spv::Error>),

    #[cfg(feature = "naga-glsl")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga-glsl")))]
    #[error("Naga GLSL error\n{0}")]
    NagaGlsl(Arc<::naga::back::glsl::Error>),
}

#[docfg(feature = "naga")]
impl From<::naga::front::spv::Error> for CompilerError {
    fn from(value: ::naga::front::spv::Error) -> Self {
        Self::NagaSpv(Arc::new(value))
    }
}

#[docfg(feature = "naga-glsl")]
impl From<::naga::back::glsl::Error> for CompilerError {
    fn from(value: ::naga::back::glsl::Error) -> Self {
        Self::NagaGlsl(Arc::new(value))
    }
}
