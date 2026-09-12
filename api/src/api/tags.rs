use std::{collections::HashMap, sync::Arc};

use crate::domain::tags::{Tag, TagView};
use crate::schema::{tag_documents, tags};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ApiErrorContext, ResourceList, validate_slug};

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewTag {
    slug: String,
    name: String,
    color: String,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct TagChangeset {
    name: Option<String>,
    color: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagSortField {
    Id,
    Slug,
    Name,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListTagsQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive search of slug and name.
    pub q: Option<String>,
    /// Sort field.
    pub sf: Option<TagSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
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
        .api_context("Failed to fetch tag")?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "tags",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact tag slug")),
    responses(
        (status = 200, description = "Tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
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
        .api_context("Failed to fetch tag")?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "tags",
    security(("bearer" = [])),
    request_body = NewTag,
    responses(
        (status = 200, description = "Created tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewTag>,
) -> Result<Json<Tag>, ApiError> {
    validate_slug(&input.slug)?;

    let inserted: Tag = diesel::insert_into(tags::table)
        .values((
            &input,
            tags::created_by.eq(user.user_id),
            tags::updated_by.eq(user.user_id),
        ))
        .returning(Tag::as_returning())
        .get_result(&mut db)
        .await
        .api_context("Failed to create tag")?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    request_body = TagChangeset,
    responses(
        (status = 200, description = "Updated tag", body = Tag),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
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
        .api_context("Failed to update tag")?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "tags",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Tag ID")),
    responses(
        (status = 200, description = "Tag deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Tag not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(tags::table.filter(tags::id.eq(id)))
        .execute(&mut db)
        .await
        .api_context("Failed to delete tag")?;

    if affected == 0 {
        return Err(ApiError::not_found("Tag not found"));
    }

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "tags",
    security(("bearer" = [])),
    params(ListTagsQuery),
    responses(
        (status = 200, description = "Paginated tag list with document counts", body = ResourceList<TagView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListTagsQuery>,
) -> Result<Json<ResourceList<TagView>>, ApiError> {
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
        .api_context("Failed to count tags")?;

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

    let tags = query
        .limit(per_page)
        .offset(offset)
        .select(Tag::as_select())
        .load::<Tag>(&mut db)
        .await
        .api_context("Failed to list tags")?;
    let tag_ids: Vec<i64> = tags.iter().map(|tag| tag.id).collect();

    let mut document_counts_by_tag: HashMap<i64, i64> = HashMap::new();
    if !tag_ids.is_empty() {
        let document_count_rows: Vec<(i64, i64)> = tag_documents::table
            .filter(tag_documents::tag_id.eq_any(&tag_ids))
            .group_by(tag_documents::tag_id)
            .select((
                tag_documents::tag_id,
                diesel::dsl::count(tag_documents::document_id),
            ))
            .load::<(i64, i64)>(&mut db)
            .await
            .api_context("Failed to count tag documents")?;

        for (tag_id, document_count) in document_count_rows {
            document_counts_by_tag.insert(tag_id, document_count);
        }
    }

    let items = tags
        .into_iter()
        .map(|tag| TagView {
            id: tag.id,
            slug: tag.slug,
            name: tag.name,
            color: tag.color,
            document_count: document_counts_by_tag.get(&tag.id).cloned().unwrap_or(0),
            created_at: tag.created_at,
            created_by: tag.created_by,
            updated_at: tag.updated_at,
            updated_by: tag.updated_by,
        })
        .collect();

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
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
