#![feature(async_fn_in_trait, pin_deref_mut, exit_status_error)]

use axum::Router;
use std::path::Path;
use tower_http::services::ServeFile;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{info, Level};

pub mod api;
pub mod compiler;
pub mod tmp;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let _ = color_eyre::install();
    tracing_subscriber::fmt::init();

    let html_path = AsRef::<Path>::as_ref(module_path!())
        .parent()
        .unwrap()
        .with_file_name("index.html");

    info!("Path of the HTML file: {}", html_path.display());

    // build our application with a single route
    let app = Router::new()
        .nest_service("/", ServeFile::new(html_path))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();

    return Ok(());
}
