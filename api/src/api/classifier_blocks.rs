use std::sync::Arc;

use crate::AppState;
use crate::domain::classifier_blocks::{ClassifierBlock, ClassifierRules};
use crate::schema::classifier_blocks;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Deserialize)]
struct NewClassifierBlock {
    name: String,
    description: Option<String>,
    enabled: bool,
    order: i32,
    rules: ClassifierRules,
}

#[derive(Debug, Deserialize)]
struct ClassifierBlockChangeset {
    name: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
    order: Option<i32>,
    rules: Option<ClassifierRules>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct ListClassifierBlocksQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub q: Option<String>,
    pub sf: Option<ClassifierBlockSortField>,
    pub sd: Option<bool>,
}

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

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewClassifierBlock>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let inserted: ClassifierBlock = diesel::insert_into(classifier_blocks::table)
        .values((
            classifier_blocks::name.eq(input.name),
            classifier_blocks::description.eq(input.description),
            classifier_blocks::enabled.eq(input.enabled),
            classifier_blocks::order.eq(input.order),
            classifier_blocks::rules.eq(diesel_json::Json(input.rules)),
            classifier_blocks::created_by.eq(user.user_id),
            classifier_blocks::updated_by.eq(user.user_id),
        ))
        .returning(ClassifierBlock::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create classifier_block"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<ClassifierBlockChangeset>,
) -> Result<Json<ClassifierBlock>, ApiError> {
    let patch = input;

    let updated: ClassifierBlock =
        diesel::update(classifier_blocks::table.filter(classifier_blocks::id.eq(id)))
            .set((
                patch.name.map(|value| classifier_blocks::name.eq(value)),
                patch
                    .description
                    .map(|value| classifier_blocks::description.eq(value)),
                patch
                    .enabled
                    .map(|value| classifier_blocks::enabled.eq(value)),
                patch.order.map(|value| classifier_blocks::order.eq(value)),
                patch
                    .rules
                    .map(|value| classifier_blocks::rules.eq(diesel_json::Json(value))),
                classifier_blocks::updated_at.eq(diesel::dsl::now),
                classifier_blocks::updated_by.eq(user.user_id),
            ))
            .returning(ClassifierBlock::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update classifier_block"))?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(classifier_blocks::table.filter(classifier_blocks::id.eq(id)))
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete classifier_block"))?;

    if affected == 0 {
        return Err(ApiError::not_found("Classifier block not found"));
    }

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListClassifierBlocksQuery>,
) -> Result<Json<ResourceList<ClassifierBlock>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
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

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
}
