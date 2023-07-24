use docfg::docfg;
use std::sync::Arc;

#[cfg(feature = "naga")]
pub mod naga;

#[cfg(feature = "spirv_cross")]
pub mod spvc;

#[cfg(feature = "spirv-tools")]
pub mod spvt;

#[derive(Debug, Clone, thiserror::Error)]
pub enum CompilerError {
    #[cfg(feature = "spirv_cross")]
    #[cfg_attr(docsrs, doc(cfg(feature = "spirv_cross")))]
    #[error("Spirv cross error")]
    SpirvCross(#[from] spirv_cross::ErrorCode),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga validation error")]
    NagaValidation(#[from] ::naga::WithSpan<::naga::valid::ValidationError>),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga SPIR-V error\n{0}")]
    NagaSpv(Arc<::naga::front::spv::Error>),

    #[cfg(feature = "naga-glsl")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga-glsl")))]
    #[error("Naga GLSL error\n{0}")]
    NagaGlsl(Arc<::naga::back::glsl::Error>),

    #[cfg(feature = "naga-hlsl")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga-hlsl")))]
    #[error("Naga HLSL error\n{0}")]
    NagaHlsl(Arc<::naga::back::hlsl::Error>),

    #[cfg(feature = "naga-msl")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga-msl")))]
    #[error("Naga MSL error\n{0}")]
    NagaMsl(Arc<::naga::back::msl::Error>),

    #[cfg(feature = "naga-wgsl")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga-wgsl")))]
    #[error("Naga WGSL error\n{0}")]
    NagaWgsl(Arc<::naga::back::wgsl::Error>),
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

#[docfg(feature = "naga-hlsl")]
impl From<::naga::back::hlsl::Error> for CompilerError {
    fn from(value: ::naga::back::hlsl::Error) -> Self {
        Self::NagaHlsl(Arc::new(value))
    }
}

#[docfg(feature = "naga-msl")]
impl From<::naga::back::msl::Error> for CompilerError {
    fn from(value: ::naga::back::msl::Error) -> Self {
        Self::NagaMsl(Arc::new(value))
    }
}

#[docfg(feature = "naga-wgsl")]
impl From<::naga::back::wgsl::Error> for CompilerError {
    fn from(value: ::naga::back::wgsl::Error) -> Self {
        Self::NagaWgsl(Arc::new(value))
    }
}
