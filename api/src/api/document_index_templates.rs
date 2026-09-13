use std::sync::Arc;

use crate::application::document_index_templates::{
    DocumentIndexTemplateChangeset, ListDocumentIndexTemplatesQuery, NewDocumentIndexTemplate,
    create_document_index_template, delete_document_index_template, get_document_index_template,
    list_document_index_templates, update_document_index_template,
};
use crate::domain::document_indexes::DocumentIndexTemplate;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
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
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Document Index Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    let row = get_document_index_template(&mut db, document_index_id, id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{document_index_id}/templates",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("document_index_id" = i64, Path, description = "Document Index ID")),
    request_body = NewDocumentIndexTemplate,
    responses(
        (status = 200, description = "Created Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
        (status = 422, description = "Invalid document index or parent template", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Json(input): Json<NewDocumentIndexTemplate>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    let inserted =
        create_document_index_template(&mut db, user.user_id, document_index_id, input).await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    request_body = DocumentIndexTemplateChangeset,
    responses(
        (status = 200, description = "Updated Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
        (status = 422, description = "Invalid parent template", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
    Json(input): Json<DocumentIndexTemplateChangeset>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    let updated =
        update_document_index_template(&mut db, user.user_id, document_index_id, id, input).await?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    delete_document_index_template(&mut db, document_index_id, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/templates",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ListDocumentIndexTemplatesQuery,
    ),
    responses(
        (status = 200, description = "Paginated Template list", body = ResourceList<DocumentIndexTemplate>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Query(params): Query<ListDocumentIndexTemplatesQuery>,
) -> Result<Json<ResourceList<DocumentIndexTemplate>>, ApiError> {
    let list = list_document_index_templates(&mut db, document_index_id, params).await?;

    Ok(Json(list))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
}
