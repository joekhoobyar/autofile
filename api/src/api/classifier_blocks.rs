use std::sync::Arc;

use crate::application::classifier_blocks::{
    UpdateClassifierBlockInput, create_classifier_block, delete_classifier_block,
    reorder_classifier_block, update_classifier_block,
};
use crate::application::classifier_rule_validation::{
    ClassifierRulesValidation, validate_classifier_rules,
};
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierRules};
use crate::schema::classifier_blocks;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
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

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ClassifierBlockSortField {
    Id,
    Name,
    Description,
    Enabled,
    Order,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListClassifierBlocksQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 1000).
    pub per_page: Option<i64>,
    /// Case-insensitive search of name and description.
    pub q: Option<String>,
    /// Sort field.
    pub sf: Option<ClassifierBlockSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
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
    let row = classifier_blocks::table
        .find(id)
        .select(ClassifierBlock::as_select())
        .first::<ClassifierBlock>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch classifier_block"))?;

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
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(1000).clamp(1, 1000);
    let offset = (page - 1) * per_page;

    let base_filter = || -> classifier_blocks::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = classifier_blocks::table.into_boxed();

        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                classifier_blocks::name
                    .ilike(pattern.clone())
                    .or(classifier_blocks::description.ilike(pattern)),
            );
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count classifier_blocks"))?;

    let mut query: classifier_blocks::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(ClassifierBlockSortField::Name), Some(true)) => {
            query.order((classifier_blocks::name.desc(), classifier_blocks::id.asc()))
        }
        (Some(ClassifierBlockSortField::Name), _) => {
            query.order((classifier_blocks::name.asc(), classifier_blocks::id.asc()))
        }
        (Some(ClassifierBlockSortField::Description), Some(true)) => query.order((
            classifier_blocks::description.desc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::Description), _) => query.order((
            classifier_blocks::description.asc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::Enabled), Some(true)) => query.order((
            classifier_blocks::enabled.desc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::Enabled), _) => query.order((
            classifier_blocks::enabled.asc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::Order), Some(true)) => {
            query.order((classifier_blocks::order.desc(), classifier_blocks::id.asc()))
        }
        (Some(ClassifierBlockSortField::Order), _) => {
            query.order((classifier_blocks::order.asc(), classifier_blocks::id.asc()))
        }
        (Some(ClassifierBlockSortField::CreatedAt), Some(true)) => query.order((
            classifier_blocks::created_at.desc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::CreatedAt), _) => query.order((
            classifier_blocks::created_at.asc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::UpdatedAt), Some(true)) => query.order((
            classifier_blocks::updated_at.desc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::UpdatedAt), _) => query.order((
            classifier_blocks::updated_at.asc(),
            classifier_blocks::id.asc(),
        )),
        (Some(ClassifierBlockSortField::Id), Some(true)) => {
            query.order(classifier_blocks::id.desc())
        }
        _ => query.order(classifier_blocks::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(ClassifierBlock::as_select())
        .load::<ClassifierBlock>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list classifier_blocks"))?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
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
