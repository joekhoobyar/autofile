use std::sync::Arc;

use crate::application::document_types::{
    DocumentTypeChangeset, ListDocumentTypesQuery, NewDocumentType, create_document_type,
    delete_document_type, get_document_type, get_document_type_by_slug, list_document_types,
    update_document_type,
};
use crate::domain::document_types::{DocumentType, DocumentTypeView};
use crate::shared::app_state::AppState;
use crate::shared::auth::{AdminUser, AuthUser};
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "document-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Type ID")),
    responses(
        (status = 200, description = "Document Type", body = DocumentType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Type not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentType>, ApiError> {
    let row = get_document_type(&mut db, id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "document-types",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact document type slug")),
    responses(
        (status = 200, description = "Document Type", body = DocumentType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Type not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<DocumentType>, ApiError> {
    let row = get_document_type_by_slug(&mut db, slug).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "document-types",
    security(("bearer" = [])),
    request_body = NewDocumentType,
    responses(
        (status = 200, description = "Created Document Type", body = DocumentType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentType>,
) -> Result<Json<DocumentType>, ApiError> {
    let inserted = create_document_type(&mut db, user.user_id, input).await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "document-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Type ID")),
    request_body = DocumentTypeChangeset,
    responses(
        (status = 200, description = "Updated Document Type", body = DocumentType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Document Type not found", body = ApiError),
    )
)]
async fn update(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentTypeChangeset>,
) -> Result<Json<DocumentType>, ApiError> {
    let updated = update_document_type(&mut db, user.user_id, id, input).await?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "document-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document Type ID")),
    responses(
        (status = 200, description = "Document Type deleted; its documents are reassigned to the default type"),
        (status = 400, description = "Cannot delete the default document type", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Document Type not found", body = ApiError),
    )
)]
async fn delete(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_document_type(&mut db, user.user_id, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "document-types",
    security(("bearer" = [])),
    params(ListDocumentTypesQuery),
    responses(
        (status = 200, description = "Paginated Document Type list with document counts", body = ResourceList<DocumentTypeView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentTypesQuery>,
) -> Result<Json<ResourceList<DocumentTypeView>>, ApiError> {
    Ok(Json(list_document_types(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
