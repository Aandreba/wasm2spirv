use std::panic::catch_unwind;

use crate::{
    compiler::{rust::RustCompiler, Compiler},
    Result,
};
use axum::{routing::post, Json, Router};
use color_eyre::Report;
use serde::{Deserialize, Serialize};
use spirv::MemoryModel;
use tower_http::catch_panic::CatchPanicLayer;
use vector_mapp::vec::VecMap;
use wasm2spirv::{
    config::{AddressingModel, CapabilityModel, Config},
    fg::function::FunctionConfig,
    version::TargetPlatform,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Rust,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompileBody {
    source: String,
    lang: Language,
    functions: VecMap<u32, FunctionConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompileResponse {
    wat: String,
    spv: String,
}

async fn compile(Json(body): Json<CompileBody>) -> Result<Json<CompileResponse>> {
    let wasm = match body.lang {
        Language::Rust => RustCompiler.compile(&body.source).await?,
    };

    let config = Config::builder(
        TargetPlatform::VK_1_1,
        CapabilityModel::dynamic(Vec::new()),
        None::<String>,
        AddressingModel::Logical,
        MemoryModel::GLSL450,
    )?
    .append_functions(body.functions)
    .build()?;

    let result = match catch_unwind(|| wasm2spirv::Compilation::new(config, &wasm)) {
        Ok(result) => result?,
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&'static str>() {
                return Err(Report::msg(*s).into());
            }

            if let Ok(s) = e.downcast::<String>() {
                return Err(Report::msg(*s).into());
            }

            return Err(Report::msg("Compiler panicked!").into());
        }
    };

    return Ok(CompileResponse {
        wat: wasmprinter::print_bytes(&wasm).map_err(Report::msg)?,
        spv: result.into_assembly()?,
    }
    .into());
}

pub fn router() -> Router {
    return Router::new().route("/compile", post(compile));
}
