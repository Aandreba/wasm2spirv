use crate::error::{Error, Result};
use crate::Compilation;
use docfg::docfg;

impl Compilation {
    #[cfg(feature = "spvc-glsl")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))))]
    pub fn glsl(&self) -> Result<&str> {
        use spirv_cross::{glsl, spirv};

        match self.glsl.get_or_try_init(|| {
            let mut module = spirv::Module::from_words(self.words()?);

            match spirv::Ast::<glsl::Target>::parse(&module) {
                Ok(mut ast) => {
                    let mut options = glsl::CompilerOptions::default();
                    options.vulkan_semantics = self.platform.is_vulkan();
                    options.separate_shader_objects = false;
                    ast.set_compiler_options(&options);

                    Ok::<_, Error>(
                        ast.compile()
                            .map(String::into_boxed_str)
                            .map_err(Into::into),
                    )
                }
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    #[cfg(feature = "spvc-hlsl")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))))]
    pub fn hlsl(&self) -> Result<&str> {
        use spirv_cross::{hlsl, spirv};

        match self.hlsl.get_or_try_init(|| {
            let module = spirv::Module::from_words(self.words()?);
            match spirv::Ast::<hlsl::Target>::parse(&module) {
                Ok(mut ast) => Ok::<_, Error>(
                    ast.compile()
                        .map(String::into_boxed_str)
                        .map_err(Into::into),
                ),
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    #[cfg(feature = "spvc-msl")]
    #[cfg_attr(docsrs, doc(cfg(any(feature = "spvc-msl", feature = "naga-msl"))))]
    pub fn msl(&self) -> Result<&str> {
        use spirv_cross::{msl, spirv};

        match self.msl.get_or_try_init(|| {
            let module = spirv::Module::from_words(self.words()?);
            match spirv::Ast::<msl::Target>::parse(&module) {
                Ok(mut ast) => {
                    let mut options = msl::CompilerOptions::default();
                    options.enable_point_size_builtin = true;
                    ast.set_compiler_options(&options);

                    Ok::<_, Error>(
                        ast.compile()
                            .map(String::into_boxed_str)
                            .map_err(Into::into),
                    )
                }
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }
}
