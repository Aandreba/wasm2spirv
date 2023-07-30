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
    cell::UnsafeCell,
    mem::{size_of, ManuallyDrop},
    ops::Deref,
};
use version::TargetPlatform;

// pub mod binary;
pub mod capabilities;
pub mod compilers;
pub mod config;
pub mod decorator;
pub mod error;
pub mod fg;
pub mod translation;
pub mod r#type;
pub mod version;

pub struct Compilation {
    pub platform: TargetPlatform,
    module: OnceCell<Result<Module, ParseState>>,
    #[cfg(feature = "naga")]
    naga_module:
        OnceCell<Result<(naga::Module, naga::valid::ModuleInfo), compilers::CompilerError>>,
    #[cfg(feature = "spirvcross")]
    spvc_context: OnceCell<Result<UnsafeCell<spirvcross::Context>, spirvcross::Error>>,
    #[cfg(feature = "spirv-tools")]
    target_env: spirv_tools::TargetEnv,
    assembly: OnceCell<Box<str>>,
    words: OnceCell<Box<[u32]>>,
    #[cfg(feature = "spvt-validate")]
    validate: OnceCell<Option<spirv_tools::error::Error>>,
}

impl Compilation {
    pub fn new(config: Config, bytes: &[u8]) -> Result<Self> {
        let platform = config.platform;
        #[cfg(feature = "spirv-tools")]
        let target_env = spirv_tools::TargetEnv::from(&config.platform);
        let builder = ModuleBuilder::new(config, bytes)?;
        let module = builder.translate()?.module();

        return Ok(Self {
            platform,
            module: OnceCell::with_value(Ok(module)),
            #[cfg(feature = "naga")]
            naga_module: OnceCell::new(),
            #[cfg(feature = "spirvcross")]
            spvc_context: OnceCell::new(),
            #[cfg(feature = "spirv-tools")]
            target_env,
            assembly: OnceCell::new(),
            words: OnceCell::new(),
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
            core::slice::from_raw_parts(words.as_ptr().cast(), std::mem::size_of_val(words))
        });
    }

    #[docfg(any(feature = "spvt-validate", feature = "naga-validate"))]
    #[inline]
    pub fn validate(&self) -> Result<()> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "spvt-validate")] {
                return self.spvt_validate()
            } else {
                return self.naga_validate()
            }
        }
    }

    #[docfg(any(feature = "spvc-glsl", feature = "naga-glsl"))]
    #[inline]
    pub fn glsl(&self) -> Result<String> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "spvc-glsl")] {
                return self.spvc_glsl()
            } else {
                return self.naga_glsl()
            }
        }
    }

    #[docfg(any(feature = "spvc-hlsl", feature = "naga-hlsl"))]
    #[inline]
    pub fn hlsl(&self) -> Result<String> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "spvc-hlsl")] {
                return self.spvc_hlsl()
            } else {
                return self.naga_hlsl()
            }
        }
    }

    #[docfg(any(feature = "spvc-msl", feature = "naga-msl"))]
    #[inline]
    pub fn msl(&self) -> Result<String> {
        cfg_if::cfg_if! {
            if #[cfg(feature = "spvc-msl")] {
                return self.spvc_msl()
            } else {
                return self.naga_msl()
            }
        }
    }

    #[docfg(feature = "naga-wgsl")]
    #[inline]
    pub fn wgsl(&self) -> Result<String> {
        return self.naga_wgsl();
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
