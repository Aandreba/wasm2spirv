use crate::Compilation;
use docfg::docfg;

impl Compilation {
    #[docfg(feature = "spvc-glsl")]
    pub fn glsl(&self) -> Result<&str> {
        use spirv_cross::{glsl, spirv};

        match self.glsl.get_or_try_init(|| {
            let module = spirv::Module::from_words(self.words()?);
            match spirv::Ast::<glsl::Target>::parse(&module) {
                Ok(mut ast) => Ok::<_, Error>(ast.compile().map(String::into_boxed_str)),
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    #[docfg(feature = "spvc-hlsl")]
    pub fn hlsl(&self) -> Result<&str> {
        use spirv_cross::{hlsl, spirv};

        match self.hlsl.get_or_try_init(|| {
            let module = spirv::Module::from_words(self.words()?);
            match spirv::Ast::<hlsl::Target>::parse(&module) {
                Ok(mut ast) => Ok::<_, Error>(ast.compile().map(String::into_boxed_str)),
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    #[docfg(feature = "spvc-msl")]
    pub fn msl(&self) -> Result<&str> {
        use spirv_cross::{msl, spirv};

        match self.msl.get_or_try_init(|| {
            let module = spirv::Module::from_words(self.words()?);
            match spirv::Ast::<msl::Target>::parse(&module) {
                Ok(mut ast) => Ok::<_, Error>(ast.compile().map(String::into_boxed_str)),
                Err(e) => Ok(Err(e.into())),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }
}
