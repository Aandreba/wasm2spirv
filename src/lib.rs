#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_return)]

use config::Config;
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

#[cfg(all(feature = "spvt-validate", feature = "naga-validate"))]
compile_error!("You can't select both SPIRV-Tools and Naga validators. Only one can be enabled at the same time");
#[cfg(all(feature = "spvc-glsl", feature = "naga-glsl"))]
compile_error!("You can't select both SPIRV-Cross and Naga compilers for GLSL. Only one can be enabled at the same time");
#[cfg(all(feature = "spvc-hlsl", feature = "naga-hlsl"))]
compile_error!("You can't select both SPIRV-Cross and Naga compilers for HLSL. Only one can be enabled at the same time");
#[cfg(all(feature = "spvc-msl", feature = "naga-msl"))]
compile_error!("You can't select both SPIRV-Cross and Naga compilers for MSL. Only one can be enabled at the same time");

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
    #[cfg(feature = "spvt-validate")]
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

pub(crate) fn wasm_min_f32(x: f32, y: f32) -> f32 {
    if x.is_nan() || y.is_nan() {
        return f32::NAN;
    }
    return f32::min(x, y);
}

pub(crate) fn wasm_min_f64(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    return f64::min(x, y);
}

pub(crate) fn wasm_max_f32(x: f32, y: f32) -> f32 {
    if x.is_nan() || y.is_nan() {
        return f32::NAN;
    }
    return f32::max(x, y);
}

pub(crate) fn wasm_max_f64(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    return f64::max(x, y);
}
