#![allow(clippy::needless_return)]

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

pub mod ast;
pub mod binary;
pub mod config;
pub mod decorator;
pub mod error;
pub mod translation;
pub mod r#type;
pub mod version;

pub struct Compiled {
    pub module: Module,
    assembly: OnceCell<Box<str>>,
    words: OnceCell<Box<[u32]>>,
}

impl Compiled {
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
