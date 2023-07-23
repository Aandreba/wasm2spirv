#[cfg(feature = "naga")]
pub mod naga;

#[cfg(feature = "spirv_cross")]
pub mod spvc;

#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[cfg(feature = "spirv_cross")]
    #[cfg_attr(docsrs, doc(cfg(feature = "spirv_cross")))]
    #[error("Spirv cross error")]
    SpirvCross(#[from] spirv_cross::ErrorCode),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga error")]
    NagaValidation(#[from] naga::WithSpan<naga::valid::ValidationError>),

    #[cfg(feature = "naga")]
    #[cfg_attr(docsrs, doc(cfg(feature = "naga")))]
    #[error("Naga SPIR-V error")]
    NagaSpv(#[from] naga::front::spv::Error),
}
