#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_return)]

use config::Config;
use docfg::docfg;
use error::{Error, Result};
use fg::module::ModuleBuilder;
use once_cell::unsync::OnceCell;
use rspirv::{
    binary::{Assemble, Disassemble, ParseState},
    dr::Module,
};
use serde::{Deserialize, Serialize};
use std::{
    mem::{size_of, ManuallyDrop},
    ops::Deref,
};

// #[cfg(all(feature = "spvc-glsl", feature = "naga-glsl"))]
// compile_error!("You can't select both SPIRV-Cross and Naga compilers for GLSL. Only one can be enabled at the same time");
// #[cfg(all(feature = "spvc-hlsl", feature = "naga-hlsl"))]
// compile_error!("You can't select both SPIRV-Cross and Naga compilers for HLSL. Only one can be enabled at the same time");
// #[cfg(all(feature = "spvc-msl", feature = "naga-msl"))]
// compile_error!("You can't select both SPIRV-Cross and Naga compilers for MSL. Only one can be enabled at the same time");

// pub mod binary;
pub mod compilers;
pub mod config;
pub mod decorator;
pub mod error;
pub mod fg;
pub mod translation;
pub mod r#type;
pub mod version;

pub struct Compilation {
    module: OnceCell<Result<Module, ParseState>>,
    #[cfg(feature = "naga")]
    naga_module:
        OnceCell<Result<(naga::Module, naga::valid::ModuleInfo), compilers::CompilerError>>,
    #[cfg(feature = "spirv-tools")]
    target_env: spirv_tools::TargetEnv,
    assembly: OnceCell<Box<str>>,
    words: OnceCell<Box<[u32]>>,
    #[cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
    glsl: OnceCell<Result<Box<str>, compilers::CompilerError>>,
    #[cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
    hlsl: OnceCell<Result<Box<str>, compilers::CompilerError>>,
    #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
    msl: OnceCell<Result<Box<str>, compilers::CompilerError>>,
    #[cfg(feature = "naga-wgsl")]
    wgsl: OnceCell<Result<Box<str>, compilers::CompilerError>>,
    #[cfg(feature = "spirv-tools")]
    validate: OnceCell<Option<spirv_tools::error::Error>>,
}

impl Compilation {
    pub fn new(config: Config, bytes: &[u8]) -> Result<Self> {
        #[cfg(feature = "spirv-tools")]
        let target_env = spirv_tools::TargetEnv::from(&config.platform);
        let builder = ModuleBuilder::new(config, bytes)?;
        let module = builder.translate()?.module();

        return Ok(Self {
            module: OnceCell::with_value(Ok(module)),
            #[cfg(feature = "naga")]
            naga_module: OnceCell::new(),
            #[cfg(feature = "spirv-tools")]
            target_env,
            assembly: OnceCell::new(),
            words: OnceCell::new(),
            #[cfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
            glsl: OnceCell::new(),
            #[cfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
            hlsl: OnceCell::new(),
            #[cfg(any(feature = "spvc-msl", feature = "naga-msl"))]
            msl: OnceCell::new(),
            #[cfg(feature = "naga-wgsl")]
            wgsl: OnceCell::new(),
            #[cfg(feature = "spirv-tools")]
            validate: OnceCell::new(),
        });
    }

    pub fn module(&self) -> Result<&Module> {
        match self.module.get_or_try_init(|| {
            let mut loader = rspirv::dr::Loader::new();
            match rspirv::binary::parse_words(self.words()?, &mut loader) {
                Ok(_) => Ok::<_, Error>(Ok(loader.module())),
                Err(e) => Ok(Err(e)),
            }
        })? {
            Ok(x) => Ok(x),
            Err(e) => Err(Error::msg(e.to_string())),
        }
    }

    pub fn assembly(&self) -> Result<&str> {
        self.assembly
            .get_or_try_init(|| Ok(self.module()?.disassemble().into_boxed_str()))
            .map(Deref::deref)
    }

    pub fn words(&self) -> Result<&[u32]> {
        self.words
            .get_or_try_init(|| Ok(self.module()?.assemble().into_boxed_slice()))
            .map(Deref::deref)
    }

    pub fn bytes(&self) -> Result<&[u8]> {
        let words = self.words()?;
        return Ok(unsafe {
            core::slice::from_raw_parts(words.as_ptr().cast(), size_of::<u32>() * words.len())
        });
    }

    #[docfg(feature = "spirv-tools")]
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
        use spirv_tools::opt::Optimizer;

        let optimizer = spirv_tools::opt::create(Some(self.target_env));
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

    pub fn into_assembly(self) -> Result<String> {
        if self.assembly.get().is_some() {
            let str = unsafe { self.assembly.into_inner().unwrap_unchecked() };
            Ok(str.into_string())
        } else {
            Ok(self.module()?.disassemble())
        }
    }

    pub fn into_words(self) -> Result<Vec<u32>> {
        if self.words.get().is_some() {
            let str = unsafe { self.words.into_inner().unwrap_unchecked() };
            Ok(str.into_vec())
        } else {
            Ok(self.module()?.assemble())
        }
    }

    pub fn into_bytes(self) -> Result<Vec<u8>> {
        let mut words = ManuallyDrop::new(self.into_words()?);
        return Ok(unsafe {
            Vec::from_raw_parts(
                words.as_mut_ptr().cast(),
                size_of::<u32>() * words.len(),
                size_of::<u32>() * words.capacity(),
            )
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Str<'a> {
    Owned(Box<str>),
    Borrowed(&'a str),
}

impl<'a> Deref for Str<'a> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        match self {
            Str::Owned(x) => x,
            Str::Borrowed(x) => x,
        }
    }
}

impl<'a> Serialize for Str<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.deref().serialize(serializer)
    }
}

impl<'a, 'de> Deserialize<'de> for Str<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|x| Self::Owned(x.into_boxed_str()))
    }
}

impl<'a> From<String> for Str<'a> {
    fn from(value: String) -> Self {
        Self::Owned(value.into_boxed_str())
    }
}

impl<'a> From<Box<str>> for Str<'a> {
    fn from(value: Box<str>) -> Self {
        Self::Owned(value)
    }
}

impl<'a> From<&'a str> for Str<'a> {
    fn from(value: &'a str) -> Self {
        Self::Borrowed(value)
    }
}

impl<'a> From<Str<'a>> for String {
    fn from(value: Str<'a>) -> Self {
        match value {
            Str::Owned(x) => x.into_string(),
            Str::Borrowed(x) => String::from(x),
        }
    }
}

#[cfg(feature = "spirv-tools")]
fn clone_diagnostics(diag: &spirv_tools::error::Diagnostic) -> spirv_tools::error::Diagnostic {
    return spirv_tools::error::Diagnostic {
        line: diag.line,
        column: diag.column,
        index: diag.index,
        message: diag.message.clone(),
        is_text: diag.is_text,
    };
}

#[cfg(feature = "spirv-tools")]
fn clone_error(err: &spirv_tools::error::Error) -> spirv_tools::error::Error {
    return spirv_tools::error::Error {
        inner: err.inner,
        diagnostic: err.diagnostic.as_ref().map(clone_diagnostics),
    };
}

#[cfg(feature = "spirv-tools")]
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
