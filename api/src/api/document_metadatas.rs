use std::sync::Arc;

use crate::application::document_metadatas::{
    NewDocumentMetadata, delete_document_metadata, get_document_metadata, list_document_metadatas,
    upsert_document_metadatas,
};
use crate::domain::document_metadatas::DocumentMetadata;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;

use axum::extract::State;

use axum::{Json, extract::Path};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{document_id}/metadata/{metadata_type_id}",
    tag = "document-metadata",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Stored metadata value", body = DocumentMetadata),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "No stored value", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<DocumentMetadata>, ApiError> {
    let row = get_document_metadata(&mut db, document_id, metadata_type_id).await?;
    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{document_id}/metadata",
    tag = "document-metadata",
    security(("bearer" = [])),
    params(("document_id" = i64, Path, description = "Document ID")),
    request_body = Vec<NewDocumentMetadata>,
    responses(
        (status = 200, description = "All stored metadata rows for the document after the upsert", body = Vec<DocumentMetadata>),
        (status = 400, description = "Invalid request, e.g. unknown field or bad date/lookup value", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 422, description = "Validation failed", body = ApiError),
    )
)]
async fn upsert(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
    Json(input): Json<Vec<NewDocumentMetadata>>,
) -> Result<Json<Vec<DocumentMetadata>>, ApiError> {
    let rows = upsert_document_metadatas(state, user.user_id, &mut db, document_id, input).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    get,
    path = "/{document_id}/metadata",
    tag = "document-metadata",
    security(("bearer" = [])),
    params(("document_id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Stored metadata rows ordered by Metadata Type ID", body = Vec<DocumentMetadata>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
) -> Result<Json<Vec<DocumentMetadata>>, ApiError> {
    let rows = list_document_metadatas(&mut db, document_id).await?;
    Ok(Json(rows))
}

#[utoipa::path(
    delete,
    path = "/{document_id}/metadata/{metadata_type_id}",
    tag = "document-metadata",
    security(("bearer" = [])),
    params(
        ("document_id" = i64, Path, description = "Document ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Stored value deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "No stored value", body = ApiError),
        (status = 409, description = "Value is required for the document type", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_document_metadata(state, &mut db, document_id, metadata_type_id).await?;
    Ok(Json(()))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(upsert))
        .routes(routes!(get_by_ids))
        .routes(routes!(delete_junction))
}
