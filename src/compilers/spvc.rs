use crate::error::Result;
use crate::Compilation;
use docfg::docfg;
use spirvcross::Context;
use std::cell::UnsafeCell;

impl Compilation {
    #[cfg(feature = "spvc-glsl")]
    pub fn spvc_glsl(&self) -> Result<String> {
        use spirvcross::{compiler::GlslCompiler, Compiler};

        let res = GlslCompiler::new(self.spvc_context()?, self.words()?)?
            .vulkan_semantics(self.platform.is_vulkan())?
            .compile()?;

        return Ok(res);
    }

    #[docfg(feature = "spvc-hlsl")]
    pub fn spvc_hlsl(&self) -> Result<String> {
        use spirvcross::{compiler::HlslCompiler, Compiler};

        let res = HlslCompiler::new(self.spvc_context()?, self.words()?)?.compile()?;
        return Ok(res);
    }

    #[docfg(feature = "spvc-msl")]
    pub fn spvc_msl(&self) -> Result<String> {
        use spirvcross::{compiler::MslCompiler, Compiler};

        let res = MslCompiler::new(self.spvc_context()?, self.words()?)?
            .enable_point_size_builtin(true)?
            .compile()?;

        return Ok(res);
    }

    fn spvc_context(&self) -> Result<&mut Context, spirvcross::Error> {
        return match self
            .spvc_context
            .get_or_init(|| Context::new().map(UnsafeCell::new))
        {
            Ok(ctx) => unsafe { Ok(&mut *ctx.get()) },
            Err(e) => Err(e.clone()),
        };
    }
}
