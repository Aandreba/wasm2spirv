#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(clippy::needless_return)]

use config::Config;
use docfg::docfg;
use error::Result;
use fg::module::ModuleBuilder;
use once_cell::unsync::OnceCell;
use rspirv::{
    binary::{Assemble, Disassemble},
    dr::Module,
};
use serde::{Deserialize, Serialize};
use std::{
    mem::{size_of, ManuallyDrop},
    ops::Deref,
};

pub mod binary;
pub mod config;
pub mod decorator;
pub mod error;
pub mod fg;
pub mod translation;
pub mod r#type;
pub mod version;

pub struct Compilation {
    pub module: Module,
    #[cfg(feature = "spirv-tools")]
    target_env: spirv_tools::TargetEnv,
    assembly: OnceCell<Box<str>>,
    words: OnceCell<Box<[u32]>>,
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
            module,
            #[cfg(feature = "spirv-tools")]
            target_env,
            assembly: OnceCell::new(),
            words: OnceCell::new(),
            #[cfg(feature = "spirv-tools")]
            validate: OnceCell::new(),
        });
    }

    pub fn assembly(&self) -> &str {
        self.assembly
            .get_or_init(|| self.module.disassemble().into_boxed_str())
    }

    pub fn words(&self) -> &[u32] {
        self.words
            .get_or_init(|| self.module.assemble().into_boxed_slice())
    }

    pub fn bytes(&self) -> &[u8] {
        let words = self.words();
        unsafe {
            core::slice::from_raw_parts(words.as_ptr().cast(), size_of::<u32>() * words.len())
        }
    }

    #[docfg(feature = "spirv-tools")]
    pub fn validate(&self) -> Result<(), spirv_tools::error::Error> {
        use spirv_tools::val::Validator;

        let res = self.validate.get_or_init(|| {
            let validator = spirv_tools::val::create(Some(self.target_env));
            validator.validate(self.words(), None).err()
        });

        return match res {
            Some(err) => Err(clone_error(err)),
            None => Ok(()),
        };
    }

    pub fn into_assembly(self) -> String {
        match self.assembly.into_inner() {
            Some(str) => str.into_string(),
            None => self.module.disassemble(),
        }
    }

    pub fn into_words(self) -> Vec<u32> {
        match self.words.into_inner() {
            Some(str) => str.into_vec(),
            None => self.module.assemble(),
        }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        let mut words = ManuallyDrop::new(self.into_words());
        return unsafe {
            Vec::from_raw_parts(
                words.as_mut_ptr().cast(),
                size_of::<u32>() * words.len(),
                size_of::<u32>() * words.capacity(),
            )
        };
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
