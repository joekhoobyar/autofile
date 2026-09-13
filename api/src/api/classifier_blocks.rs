use std::sync::Arc;

use crate::application::classifier_blocks::{
    ListClassifierBlocksQuery, UpdateClassifierBlockInput, create_classifier_block,
    delete_classifier_block, get_classifier_block, list_classifier_blocks,
    reorder_classifier_block, update_classifier_block,
};
use crate::application::classifier_rule_validation::{
    ClassifierRulesValidation, validate_classifier_rules,
};
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierRules};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
struct NewClassifierBlock {
    name: String,
    description: Option<String>,
    enabled: bool,
    rules: ClassifierRules,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
struct ClassifierBlockChangeset {
    name: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    rules: Option<ClassifierRules>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
struct ReorderClassifierBlock {
    order: i32,
}

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Classifier Block ID")),
    responses(
        (status = 200, description = "Classifier Block", body = ClassifierBlock),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Classifier Block not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let row = get_classifier_block(&mut db, id).await?;
    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    request_body = NewClassifierBlock,
    responses(
        (status = 200, description = "Created Classifier Block", body = ClassifierBlock),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 422, description = "Invalid rules", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewClassifierBlock>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let inserted = create_classifier_block(
        &mut db,
        user.user_id,
        input.name,
        input.description,
        input.enabled,
        input.rules,
    )
    .await?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Classifier Block ID")),
    request_body = ClassifierBlockChangeset,
    responses(
        (status = 200, description = "Updated Classifier Block", body = ClassifierBlock),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Classifier Block not found", body = ApiError),
        (status = 422, description = "Invalid rules", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<ClassifierBlockChangeset>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let updated = update_classifier_block(
        &mut db,
        user.user_id,
        id,
        UpdateClassifierBlockInput {
            name: input.name,
            description: input.description,
            enabled: input.enabled,
            rules: input.rules,
        },
    )
    .await?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Classifier Block ID")),
    responses(
        (status = 200, description = "Classifier Block deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Classifier Block not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_classifier_block(&mut db, id).await?;

    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/reorder",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Classifier Block ID")),
    request_body = ReorderClassifierBlock,
    responses(
        (status = 200, description = "Reordered Classifier Block", body = ClassifierBlock),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Classifier Block not found", body = ApiError),
    )
)]
async fn reorder(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<ReorderClassifierBlock>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let reordered = reorder_classifier_block(&mut db, user.user_id, id, input.order).await?;

    Ok(Json(reordered))
}

#[utoipa::path(
    post,
    path = "/validate",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    request_body = ClassifierRules,
    responses(
        (status = 200, description = "Validation result with issues and pattern capture counts", body = ClassifierRulesValidation),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn validate_rules(
    _user: AuthUser,
    Json(rules): Json<ClassifierRules>,
) -> Json<ClassifierRulesValidation> {
    Json(validate_classifier_rules(&rules))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "classifier-blocks",
    security(("bearer" = [])),
    params(ListClassifierBlocksQuery),
    responses(
        (status = 200, description = "Paginated Classifier Block list", body = ResourceList<ClassifierBlock>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListClassifierBlocksQuery>,
) -> Result<Json<ResourceList<ClassifierBlock>>, ApiError> {
    Ok(Json(list_classifier_blocks(&mut db, params).await?))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(validate_rules))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(reorder))
}
