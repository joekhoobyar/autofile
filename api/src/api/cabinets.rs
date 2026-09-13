use std::sync::Arc;

use crate::application::cabinets::{
    CabinetChangeset, ListCabinetsQuery, NewCabinet, create_cabinet, delete_cabinet, get_cabinet,
    get_cabinet_by_slug, list_cabinets, update_cabinet,
};
use crate::domain::cabinets::{Cabinet, CabinetView};
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
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    responses(
        (status = 200, description = "Cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Cabinet>, ApiError> {
    let row = get_cabinet(&mut db, id).await?;
    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact cabinet slug")),
    responses(
        (status = 200, description = "Cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<Cabinet>, ApiError> {
    let row = get_cabinet_by_slug(&mut db, slug).await?;
    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "cabinets",
    security(("bearer" = [])),
    request_body = NewCabinet,
    responses(
        (status = 200, description = "Created cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug or parent cabinet", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewCabinet>,
) -> Result<Json<Cabinet>, ApiError> {
    let inserted = create_cabinet(&mut db, user.user_id, input).await?;
    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    request_body = CabinetChangeset,
    responses(
        (status = 200, description = "Updated cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
        (status = 422, description = "Invalid parent cabinet", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<CabinetChangeset>,
) -> Result<Json<Cabinet>, ApiError> {
    let updated = update_cabinet(&mut db, user.user_id, id, input).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    responses(
        (status = 200, description = "Cabinet deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_cabinet(&mut db, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "cabinets",
    security(("bearer" = [])),
    params(ListCabinetsQuery),
    responses(
        (status = 200, description = "Paginated cabinet list with document counts", body = ResourceList<CabinetView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListCabinetsQuery>,
) -> Result<Json<ResourceList<CabinetView>>, ApiError> {
    Ok(Json(list_cabinets(&mut db, params).await?))
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
