use crate::error::Result;
use crate::Compilation;
use docfg::docfg;
use spirvcross::Context;
use std::cell::UnsafeCell;

impl Compilation {
    #[cfg(feature = "spvc-glsl")]
    pub fn spvc_glsl(&self) -> Result<String> {
        use spirvcross::{compiler::GlslCompiler, Compiler};

        let ctx = self.spvc_context()?;
        let res = GlslCompiler::new(ctx, self.words()?)?
            .vulkan_semantics(self.platform.is_vulkan())?
            .compile()?;

        ctx.release_allocations();
        return Ok(res);
    }

    #[docfg(feature = "spvc-hlsl")]
    pub fn spvc_hlsl(&self) -> Result<String> {
        use spirvcross::{compiler::HlslCompiler, Compiler};

        let ctx = self.spvc_context()?;
        let res = HlslCompiler::new(ctx, self.words()?)?.compile()?;
        ctx.release_allocations();
        return Ok(res);
    }

    #[docfg(feature = "spvc-msl")]
    pub fn spvc_msl(&self) -> Result<String> {
        use spirvcross::{compiler::MslCompiler, Compiler};

        let ctx = self.spvc_context()?;
        let res = MslCompiler::new(ctx, self.words()?)?
            .enable_point_size_builtin(true)?
            .compile()?;

        ctx.release_allocations();
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
