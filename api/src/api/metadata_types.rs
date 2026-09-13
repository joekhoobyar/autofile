use std::sync::Arc;

use crate::application::metadata_types::{
    ListMetadataTypeValuesQuery, ListMetadataTypesQuery, MetadataTypeChangeset, NewMetadataType,
    create_metadata_type, delete_metadata_type, get_metadata_type, get_metadata_type_by_slug,
    list_metadata_type_values, list_metadata_types, update_metadata_type,
};
use crate::domain::metadata_types::MetadataType;
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
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    responses(
        (status = 200, description = "Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<MetadataType>, ApiError> {
    let row = get_metadata_type(&mut db, id).await?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact metadata type slug")),
    responses(
        (status = 200, description = "Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<MetadataType>, ApiError> {
    let row = get_metadata_type_by_slug(&mut db, slug).await?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/{id}/values",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "Metadata Type ID"),
        ListMetadataTypeValuesQuery,
    ),
    responses(
        (status = 200, description = "Distinct stored values for string metadata types", body = Vec<String>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
        (status = 422, description = "Not a string metadata type", body = ApiError),
    )
)]
pub async fn list_values(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Query(params): Query<ListMetadataTypeValuesQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    let values = list_metadata_type_values(&mut db, id, params).await?;

    Ok(Json(values))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "metadata-types",
    security(("bearer" = [])),
    request_body = NewMetadataType,
    responses(
        (status = 200, description = "Created Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewMetadataType>,
) -> Result<Json<MetadataType>, ApiError> {
    let inserted = create_metadata_type(&mut db, user.user_id, input).await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    request_body = MetadataTypeChangeset,
    responses(
        (status = 200, description = "Updated Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
async fn update(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<MetadataTypeChangeset>,
) -> Result<Json<MetadataType>, ApiError> {
    let updated = update_metadata_type(&mut db, user.user_id, id, input).await?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    responses(
        (status = 200, description = "Metadata Type deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
        (status = 409, description = "Metadata Type is still in use", body = ApiError),
    )
)]
async fn delete(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_metadata_type(&mut db, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(ListMetadataTypesQuery),
    responses(
        (status = 200, description = "Paginated Metadata Type list", body = ResourceList<MetadataType>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListMetadataTypesQuery>,
) -> Result<Json<ResourceList<MetadataType>>, ApiError> {
    Ok(Json(list_metadata_types(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(list_values))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
