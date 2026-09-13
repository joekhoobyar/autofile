use std::sync::Arc;

use crate::application::document_types_metadata_types::{
    DocumentTypeMetadataTypeChangeset, DocumentTypeNewMetadataTypeInput,
    ListDocumentTypesMetadataTypesQuery, NewDocumentTypeMetadataType,
    create_document_type_metadata_type, delete_document_type_metadata_type,
    get_document_type_metadata_type, list_document_types_metadata_types,
    save_document_type_metadata_types, update_document_type_metadata_type,
};
use crate::domain::document_types_metadata_types::DocumentTypeMetadataType;
use crate::shared::app_state::AppState;
use crate::shared::auth::{AdminUser, AuthUser};
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Association", body = DocumentTypeMetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let row = get_document_type_metadata_type(&mut db, document_type_id, metadata_type_id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    request_body = NewDocumentTypeMetadataType,
    responses(
        (status = 200, description = "Created association", body = DocumentTypeMetadataType),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 409, description = "Association already exists", body = ApiError),
        (status = 422, description = "Unknown document or metadata type", body = ApiError),
    )
)]
async fn create(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentTypeMetadataType>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let inserted = create_document_type_metadata_type(&mut db, input).await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    request_body = DocumentTypeMetadataTypeChangeset,
    responses(
        (status = 200, description = "Updated association", body = DocumentTypeMetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn update(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
    Json(input): Json<DocumentTypeMetadataTypeChangeset>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let updated =
        update_document_type_metadata_type(&mut db, document_type_id, metadata_type_id, input)
            .await?;

    Ok(Json(updated))
}

#[utoipa::path(
    post,
    path = "/{document_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(("document_type_id" = i64, Path, description = "Document Type ID")),
    request_body = Vec<DocumentTypeNewMetadataTypeInput>,
    responses(
        (status = 200, description = "Replacement association set; an empty array removes all associations", body = Vec<DocumentTypeMetadataType>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 422, description = "Unknown metadata type", body = ApiError),
    )
)]
async fn document_type_save(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path(document_type_id): Path<i64>,
    Json(input): Json<Vec<DocumentTypeNewMetadataTypeInput>>,
) -> Result<Json<Vec<DocumentTypeMetadataType>>, ApiError> {
    let rows = save_document_type_metadata_types(&mut db, document_type_id, input).await?;

    Ok(Json(rows))
}

#[utoipa::path(
    delete,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Association removed"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_document_type_metadata_type(&mut db, document_type_id, metadata_type_id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(ListDocumentTypesMetadataTypesQuery),
    responses(
        (status = 200, description = "Association list as a bare array (no total)", body = Vec<DocumentTypeMetadataType>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentTypesMetadataTypesQuery>,
) -> Result<Json<Vec<DocumentTypeMetadataType>>, ApiError> {
    let rows = list_document_types_metadata_types(&mut db, params).await?;

    Ok(Json(rows))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(document_type_save))
        .routes(routes!(get_by_ids))
        .routes(routes!(update))
        .routes(routes!(delete_junction))
}
