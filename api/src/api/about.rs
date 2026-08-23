use std::sync::Arc;

use axum::{
    Json, Router,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::shared::app_state::AppState;

const COPYRIGHT: &str = "Copyright 2024-2026 Joe Khoobyar";
const LICENSE_TEXT: &str = include_str!("../../../LICENSE");

#[derive(serde::Serialize)]
pub struct AboutResponse {
    name: &'static str,
    version: &'static str,
    authors: &'static str,
    license: &'static str,
    copyright: &'static str,
}

async fn get_about() -> Json<AboutResponse> {
    Json(AboutResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        authors: env!("CARGO_PKG_AUTHORS"),
        license: env!("CARGO_PKG_LICENSE"),
        copyright: COPYRIGHT,
    })
}

async fn get_license() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        LICENSE_TEXT,
    )
        .into_response()
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_about))
        .route("/license", get(get_license))
}
