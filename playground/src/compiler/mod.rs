pub mod rust;

pub trait Compiler {
    async fn compile(source: &str) -> color_eyre::Result<Vec<u8>>;
}
