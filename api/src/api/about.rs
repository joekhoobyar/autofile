use std::sync::Arc;

use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::shared::app_state::AppState;

const COPYRIGHT: &str = "Copyright 2024-2026 Joe Khoobyar";
const LICENSE_TEXT: &str = include_str!("../../../LICENSE");

#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AboutResponse {
    name: &'static str,
    version: &'static str,
    authors: &'static str,
    license: &'static str,
    copyright: &'static str,
}

#[utoipa::path(
    get,
    path = "/",
    tag = "about",
    responses(
        (status = 200, description = "Application identity and build metadata", body = AboutResponse),
    )
)]
async fn get_about() -> Json<AboutResponse> {
    Json(AboutResponse {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        authors: env!("CARGO_PKG_AUTHORS"),
        license: env!("CARGO_PKG_LICENSE"),
        copyright: COPYRIGHT,
    })
}

#[utoipa::path(
    get,
    path = "/license",
    tag = "about",
    responses(
        (status = 200, description = "Application license text", content_type = "text/plain", body = String),
    )
)]
async fn get_license() -> Response {
    (
        [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
        LICENSE_TEXT,
    )
        .into_response()
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(get_about))
        .routes(routes!(get_license))
}
