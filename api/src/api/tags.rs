use std::sync::Arc;

use crate::domain::tags::Tag;
use crate::schema::tags;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewTag {
    slug: String,
    name: String,
    color: String,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct TagChangeset {
    name: Option<String>,
    color: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagSortField {
    Id,
    Slug,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListTagsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // optional sort field
    pub sf: Option<TagSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Tag>, ApiError> {
    let row = tags::table
        .find(id)
        .select(Tag::as_select())
        .first::<Tag>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch tag"))?;

    Ok(Json(row))
}

pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<Tag>, ApiError> {
    let row = tags::table
        .filter(tags::slug.eq(slug))
        .select(Tag::as_select())
        .first::<Tag>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch tag"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewTag>,
) -> Result<Json<Tag>, ApiError> {
    let inserted: Tag = diesel::insert_into(tags::table)
        .values((
            &input,
            tags::created_by.eq(user.user_id),
            tags::updated_by.eq(user.user_id),
        ))
        .returning(Tag::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create tag"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TagChangeset>,
) -> Result<Json<Tag>, ApiError> {
    let patch = input;

    // Common assignments (no parent_id here)
    let common = (
        patch.name.map(|v| tags::name.eq(v)),
        patch.color.map(|v| tags::color.eq(v)),
        tags::updated_at.eq(Utc::now()),
        tags::updated_by.eq(user.user_id),
    );

    // Update + return the updated row in one round-trip.
    let updated: Tag = diesel::update(tags::table.filter(tags::id.eq(id)))
        .set(common)
        .returning(Tag::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update tag"))?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(tags::table.filter(tags::id.eq(id)))
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete tag"))?;

    if affected == 0 {
        return Err(ApiError::not_found("Tag not found"));
    }

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListTagsQuery>,
) -> Result<Json<ResourceList<Tag>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> tags::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let mut query = tags::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                tags::slug
                    .ilike(pattern.clone())
                    .or(tags::name.ilike(pattern.clone())),
            )
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count tags"))?;

    let mut query: tags::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(TagSortField::Slug), Some(true)) => query.order((tags::slug.desc(), tags::id.asc())),
        (Some(TagSortField::Slug), _) => query.order((tags::slug.asc(), tags::id.asc())),
        (Some(TagSortField::Name), Some(true)) => query.order((tags::name.desc(), tags::id.asc())),
        (Some(TagSortField::Name), _) => query.order((tags::name.asc(), tags::id.asc())),
        (Some(TagSortField::CreatedAt), Some(true)) => {
            query.order((tags::created_at.desc(), tags::id.asc()))
        }
        (Some(TagSortField::CreatedAt), _) => query.order((tags::created_at.asc(), tags::id.asc())),
        (Some(TagSortField::UpdatedAt), Some(true)) => {
            query.order((tags::updated_at.desc(), tags::id.asc()))
        }
        (Some(TagSortField::UpdatedAt), _) => query.order((tags::updated_at.asc(), tags::id.asc())),
        (Some(TagSortField::Id), Some(true)) => query.order(tags::id.desc()),
        _ => query.order(tags::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(Tag::as_select())
        .load::<Tag>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list tags"))?;

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
        .route("/by-slug/{slug}", get(get_by_slug))
}
