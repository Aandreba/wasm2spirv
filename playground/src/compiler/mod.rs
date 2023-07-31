pub mod rust;
pub mod zig;

pub trait Compiler {
    async fn compile(&self, source: &str) -> Result<Vec<u8>, crate::Error>;
}
