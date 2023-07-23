use crate::Compilation;
use docfg::docfg;
use naga::valid;
use std::io::Cursor;

impl Compilation {
    #[docfg(feature = "naga-glsl")]
    pub fn glsl(&self) -> Result<&str> {
        use naga::back::glsl;

        match self.glsl.get_or_try_init(|| {
            let module = self.naga_module()?;
            let info =
                valid::Validator::new(valid::ValidationFlags::all(), valid::Capabilities::all())
                    .validate(&module)?;

            let varsion = match 0 {
                _ => naga::back::glsl::Version::Desktop(450),
            };

            let options = glsl::Options {
                version,
                ..Default::default()
            };

            let mut result = Cursor::new(Vec::<u8>::new());
            let mut writer = naga::back::glsl::Writer::new(&mut result, &module, &info, &options);

            match spirv::Ast::<glsl::Target>::parse(&module) {
                Ok(mut ast) => Ok::<_, Error>(ast.compile().map(String::into_boxed_str)),
                Err(e) => Ok(Err(e)),
            }
        })? {
            Ok(str) => Ok(str),
            Err(e) => Err(Error::from(e.clone())),
        }
    }

    fn naga_module(&self) -> Result<naga::Module> {
        let module =
            naga::front::spv::parse_u8_slice(self.bytes()?, &naga::front::spv::Options::default())?;
        return Ok(module);
    }
}
