use crate::error::{Error, Result};
use crate::Compilation;
use docfg::docfg;
use once_cell::unsync::OnceCell;
use std::mem::ManuallyDrop;

impl Compilation {
    #[cfg(feature = "spvt-validate")]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "spvt-validate", feature = "naga-validate")))
    )]
    pub fn validate(&self) -> Result<()> {
        use spirv_tools::val::Validator;

        let res = self.validate.get_or_try_init(|| {
            let validator = spirv_tools::val::create(Some(self.target_env));
            Ok::<_, Error>(validator.validate(self.words()?, None).err())
        })?;

        return match res {
            Some(err) => Err(Error::from(clone_error(err))),
            None => Ok(()),
        };
    }

    #[docfg(feature = "spirv-tools")]
    pub fn into_optimized(self) -> Result<Self> {
        use spirv_tools::opt::{Optimizer, Passes};

        let mut optimizer = spirv_tools::opt::create(Some(self.target_env));
        let optimizer = optimizer
            .register_hlsl_legalization_passes()
            .register_performance_passes();

        let words = match optimizer.optimize(self.words()?, &mut spirv_tools_message, None)? {
            spirv_tools::binary::Binary::External(words) => AsRef::<[u32]>::as_ref(&words).into(),
            spirv_tools::binary::Binary::OwnedU32(words) => words,
            spirv_tools::binary::Binary::OwnedU8(bytes) => {
                match bytes.as_ptr().align_offset(core::mem::align_of::<u32>()) {
                    0 if bytes.len() % 4 == 0 && bytes.capacity() % 4 == 0 => unsafe {
                        let mut bytes = ManuallyDrop::new(bytes);
                        Vec::from_raw_parts(
                            bytes.as_mut_ptr().cast(),
                            bytes.len() / 4,
                            bytes.capacity() / 4,
                        )
                    },
                    _ => {
                        let mut result = Vec::with_capacity(bytes.len() / 4);
                        for chunk in bytes.chunks_exact(4) {
                            let chunk = unsafe { TryFrom::try_from(chunk).unwrap_unchecked() };
                            result.push(u32::from_ne_bytes(chunk));
                        }
                        result
                    }
                }
            }
        };

        return Ok(Self {
            module: OnceCell::new(),
            #[cfg(feature = "naga")]
            naga_module: OnceCell::new(),
            words: OnceCell::with_value(words.into_boxed_slice()),
            #[cfg(feature = "spirv-tools")]
            target_env: self.target_env,
            assembly: OnceCell::new(),
            #[cfg(feature = "spirv_cross")]
            glsl: OnceCell::new(),
            #[cfg(feature = "spirv_cross")]
            hlsl: OnceCell::new(),
            #[cfg(feature = "spirv_cross")]
            msl: OnceCell::new(),
            #[cfg(feature = "naga-wgsl")]
            wgsl: OnceCell::new(),
            #[cfg(feature = "spirv-tools")]
            validate: OnceCell::new(),
        });
    }
}

fn clone_diagnostics(diag: &spirv_tools::error::Diagnostic) -> spirv_tools::error::Diagnostic {
    return spirv_tools::error::Diagnostic {
        line: diag.line,
        column: diag.column,
        index: diag.index,
        message: diag.message.clone(),
        is_text: diag.is_text,
    };
}

fn clone_error(err: &spirv_tools::error::Error) -> spirv_tools::error::Error {
    return spirv_tools::error::Error {
        inner: err.inner,
        diagnostic: err.diagnostic.as_ref().map(clone_diagnostics),
    };
}

fn spirv_tools_message(msg: spirv_tools::error::Message) {
    match msg.level {
        spirv_tools::error::MessageLevel::Fatal
        | spirv_tools::error::MessageLevel::InternalError
        | spirv_tools::error::MessageLevel::Error => tracing::error!("{}", msg.message),
        spirv_tools::error::MessageLevel::Warning => tracing::warn!("{}", msg.message),
        spirv_tools::error::MessageLevel::Info => tracing::info!("{}", msg.message),
        spirv_tools::error::MessageLevel::Debug => tracing::debug!("{}", msg.message),
    };
}
