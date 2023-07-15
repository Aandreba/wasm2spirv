use crate::error::Result;
use std::marker::PhantomData;

pub struct WasmInfo<'a> {
    _phtm: PhantomData<&'a ()>,
}

impl<'a> WasmInfo<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let types = wasmparser::validate(bytes)?;
        todo!()
    }
}
