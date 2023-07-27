#![feature(
    async_fn_in_trait,
    impl_trait_in_assoc_type,
    pin_deref_mut,
    duration_constants,
    exit_status_error
)]

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Router;
use color_eyre::Report;
use std::fmt::{Debug, Display};
use std::net::SocketAddr;
use std::path::Path;
use tower_http::services::ServeDir;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::{error, info, Level};

pub mod api;
pub mod compiler;
pub mod rate_limit;
pub mod tmp;

pub type Result<T, E = Error> = ::std::result::Result<T, E>;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    let _ = color_eyre::install();
    tracing_subscriber::fmt::init();

    let html_path = AsRef::<Path>::as_ref(module_path!())
        .parent()
        .unwrap()
        .join("web");

    info!("Path of the HTML file: {}", html_path.display());

    // build our application with a single route
    let app = Router::new()
        .nest("/api", api::router())
        .nest_service(
            "/",
            ServeDir::new(html_path).append_index_html_on_directories(false),
        )
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        );

    axum::Server::bind(&"0.0.0.0:8080".parse().unwrap())
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();

    return Ok(());
}

#[derive(Debug)]
#[repr(transparent)]
pub struct Error(pub Report);

impl Error {
    pub fn msg(message: impl Display + Debug + Send + Sync + 'static) -> Error {
        return Self(Report::msg(message));
    }
}

impl<T: Into<Report>> From<T> for Error {
    #[inline]
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        error!("{}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            self.0.root_cause().to_string(),
        )
            .into_response()
    }
}
