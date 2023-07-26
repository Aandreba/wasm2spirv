use axum::response::IntoResponse;

pub mod rust;

pub trait Compiler {
    type Error: IntoResponse;

    async fn compile(&self, source: &str) -> Result<Vec<u8>, Self::Error>;
}
