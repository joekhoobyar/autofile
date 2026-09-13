use std::sync::Arc;

use crate::application::cabinet_documents::{
    ListCabinetDocumentsQuery, NewCabinetDocument, delete_cabinet_document,
    delete_cabinet_documents, get_cabinet_document, list_cabinet_documents,
    upsert_cabinet_documents,
};
use crate::domain::cabinet_documents::CabinetDocument;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::extract::State;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{cabinet_id}/documents/{document_id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Cabinet-document association", body = CabinetDocument),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((cabinet_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<CabinetDocument>, ApiError> {
    let row = get_cabinet_document(&mut db, cabinet_id, document_id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{cabinet_id}/documents",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("cabinet_id" = i64, Path, description = "Cabinet ID")),
    request_body = Vec<NewCabinetDocument>,
    responses(
        (status = 200, description = "Created or updated associations", body = Vec<CabinetDocument>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 422, description = "Unprocessable request", body = ApiError),
    )
)]
async fn upsert(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(cabinet_id): Path<i64>,
    Json(input): Json<Vec<NewCabinetDocument>>,
) -> Result<Json<Vec<CabinetDocument>>, ApiError> {
    let items = upsert_cabinet_documents(state, &mut db, user.user_id, cabinet_id, input).await?;

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/{cabinet_id}/documents",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ListCabinetDocumentsQuery,
    ),
    responses(
        (status = 200, description = "Paginated cabinet-document associations", body = ResourceList<CabinetDocument>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListCabinetDocumentsQuery>,
) -> Result<Json<ResourceList<CabinetDocument>>, ApiError> {
    let list = list_cabinet_documents(&mut db, params).await?;

    Ok(Json(list))
}

#[utoipa::path(
    post,
    path = "/{cabinet_id}/documents/delete",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("cabinet_id" = i64, Path, description = "Cabinet ID")),
    request_body = Vec<i64>,
    responses(
        (status = 200, description = "Associations removed"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(cabinet_id): Path<i64>,
    Json(input): Json<Vec<i64>>,
) -> Result<Json<()>, ApiError> {
    delete_cabinet_documents(state, &mut db, cabinet_id, input).await?;

    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/{cabinet_id}/documents/{document_id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Association removed"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((cabinet_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_cabinet_document(state, &mut db, cabinet_id, document_id).await?;

    Ok(Json(()))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(upsert))
        .routes(routes!(delete))
        .routes(routes!(get_by_ids))
        .routes(routes!(delete_junction))
}
