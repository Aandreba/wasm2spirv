use crate::{
    compiler::{rust::RustCompiler, zig::ZigCompiler, Compiler},
    Result,
};
use axum::{routing::post, Json, Router};
use color_eyre::Report;
use serde::{Deserialize, Serialize};
use spirv::MemoryModel;
use std::{borrow::Cow, panic::catch_unwind};
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
    Zig,
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
    spv: Result<String, Cow<'static, str>>,
}

async fn compile(Json(body): Json<CompileBody>) -> Result<Json<CompileResponse>> {
    let wasm = match body.lang {
        Language::Rust => RustCompiler.compile(&body.source).await?,
        Language::Zig => ZigCompiler.compile(&body.source).await?,
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
        Ok(result) => result.map_err(|x| Cow::Owned(x.to_string())),
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&'static str>() {
                Err(Cow::Borrowed(*s))
            } else if let Ok(s) = e.downcast::<String>() {
                Err(Cow::Owned(*s))
            } else {
                Err(Cow::Borrowed("Compilation failed"))
            }
        }
    };

    return Ok(CompileResponse {
        wat: wasmprinter::print_bytes(&wasm).map_err(Report::msg)?,
        spv: result.and_then(|x| x.into_assembly().map_err(|e| Cow::Owned(e.to_string()))),
    }
    .into());
}

pub fn router() -> Router {
    return Router::new().route("/compile", post(compile));
}
