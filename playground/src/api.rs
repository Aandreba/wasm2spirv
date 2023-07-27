use crate::{
    compiler::{rust::RustCompiler, zig::ZigCompiler, Compiler},
    Result,
};
use axum::{routing::post, Json, Router};
use color_eyre::Report;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, panic::catch_unwind, time::Duration};
use wasm2spirv::config::Config;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    Wasm,
    Rust,
    Zig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilationLanguage {
    Spirv,
    Glsl,
    Hlsl,
    Msl,
    Wgsl,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompileBody {
    source: String,
    lang: Language,
    compile_lang: CompilationLanguage,
    config: Config,
    optimization_runs: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompileResponse {
    wat: String,
    result: Result<String, Cow<'static, str>>,
}

async fn compile(Json(body): Json<CompileBody>) -> Result<Json<CompileResponse>> {
    macro_rules! tri {
        ($e:expr) => {
            match catch_unwind(std::panic::AssertUnwindSafe(|| $e)) {
                Ok(Ok(x)) => Ok(x),
                Ok(Err(e)) => Err(Cow::Owned(e.to_string())),
                Err(e) => {
                    if let Some(s) = e.downcast_ref::<&'static str>() {
                        Err(Cow::Borrowed(*s))
                    } else if let Ok(s) = e.downcast::<String>() {
                        Err(Cow::Owned(*s))
                    } else {
                        Err(Cow::Borrowed("Compilation failed"))
                    }
                }
            }
        };
    }

    let wasm = match body.lang {
        Language::Wasm => wat::parse_str(&body.source)?,
        Language::Rust => RustCompiler.compile(&body.source).await?,
        Language::Zig => ZigCompiler.compile(&body.source).await?,
    };

    let wat = match body.lang {
        Language::Wasm => body.source,
        _ => wasmprinter::print_bytes(&wasm).map_err(Report::msg)?,
    };

    let mut result = tri!(wasm2spirv::Compilation::new(body.config, &wasm))
        .and_then(|result| tri!(result.validate()).map(|_| result));

    for _ in 0..u8::min(body.optimization_runs, 3) {
        result = result.and_then(|result| tri!(result.into_optimized()));
    }

    let result = result.and_then(|result| {
        tri!(match body.compile_lang {
            CompilationLanguage::Spirv => result.into_assembly(),
            CompilationLanguage::Glsl => result.naga_glsl(),
            CompilationLanguage::Hlsl => result.naga_hlsl(),
            CompilationLanguage::Msl => result.msl(),
            CompilationLanguage::Wgsl => result.naga_wgsl(),
        })
    });

    return Ok(CompileResponse { wat, result }.into());
}

pub fn router() -> Router {
    return Router::new()
        .route("/compile", post(compile))
        .route_layer(RateLimitLayer::new(1, Duration::SECOND));
}
