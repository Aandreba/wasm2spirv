use crate::error::{Error, Result};
use crate::Compilation;
use docfg::docfg;

impl Compilation {
    #[cfg(feature = "spvc-glsl")]
    pub fn spvc_glsl(&self) -> Result<String> {
        use spirv_cross::{glsl, spirv};

        let mut module = spirv::Module::from_words(self.words()?);
        let mut ast = spirv::Ast::<glsl::Target>::parse(&module)?;

        let mut options = glsl::CompilerOptions::default();
        options.vulkan_semantics = self.platform.is_vulkan();
        options.separate_shader_objects = false;
        ast.set_compiler_options(&options);

        return ast.compile().map_err(Into::into);
    }

    #[docfg(feature = "spvc-hlsl")]
    pub fn spvc_hlsl(&self) -> Result<String> {
        use spirv_cross::{hlsl, spirv};

        let module = spirv::Module::from_words(self.words()?);
        let mut ast = spirv::Ast::<hlsl::Target>::parse(&module)?;
        return ast.compile().map_err(Into::into);
    }

    #[docfg(feature = "spvc-msl")]
    pub fn spvc_msl(&self) -> Result<String> {
        use spirv_cross::{msl, spirv};

        let module = spirv::Module::from_words(self.words()?);
        let mut ast = spirv::Ast::<msl::Target>::parse(&module)?;

        let mut options = msl::CompilerOptions::default();
        options.enable_point_size_builtin = true;
        ast.set_compiler_options(&options);

        return ast.compile().map_err(Into::into);
    }
}
