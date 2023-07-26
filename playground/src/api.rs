use crate::compiler::{rust::RustCompiler, Compiler};
use axum::{routing::post, Json, Router};
use color_eyre::Report;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CompileBody {
    source: String,
    lang: Language,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompileResponse {
    wat: String,
}

pub async fn compile(Json(body): Json<CompileBody>) -> color_eyre::Result<Json<CompileResponse>> {
    let wasm = match body.lang {
        Language::Rust => RustCompiler.compile(&body.source).await?,
    };

    return Ok(CompileResponse {
        wat: wasmprinter::print_bytes(&wasm).map_err(Report::msg)?,
    }
    .into());
}

fn router() -> Router {
    return Router::new().route("/compile", post(compile));
}
