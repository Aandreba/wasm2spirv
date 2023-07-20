#![allow(clippy::needless_return)]

use once_cell::unsync::OnceCell;
use rspirv::{binary::Disassemble, dr::Module};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

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
