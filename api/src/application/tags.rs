use std::collections::HashMap;

use bb8::PooledConnection;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::domain::tags::{Tag, TagView};
use crate::schema::{tag_documents, tags};
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;
use crate::shared::slugs::validate_slug;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTag {
    pub slug: String,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = tags)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TagChangeset {
    pub name: Option<String>,
    pub color: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
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

pub async fn get_tag(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<Tag, ApiError> {
    tags::table
        .find(id)
        .select(Tag::as_select())
        .first::<Tag>(db)
        .await
        .api_context("Failed to fetch tag")
}

pub async fn get_tag_by_slug(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    slug: String,
) -> Result<Tag, ApiError> {
    tags::table
        .filter(tags::slug.eq(slug))
        .select(Tag::as_select())
        .first::<Tag>(db)
        .await
        .api_context("Failed to fetch tag")
}

pub async fn create_tag(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: NewTag,
) -> Result<Tag, ApiError> {
    validate_slug(&input.slug)?;

    diesel::insert_into(tags::table)
        .values((
            &input,
            tags::created_by.eq(user_id),
            tags::updated_by.eq(user_id),
        ))
        .returning(Tag::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to create tag")
}

pub async fn update_tag(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    id: i64,
    input: TagChangeset,
) -> Result<Tag, ApiError> {
    let common = (
        input.name.map(|v| tags::name.eq(v)),
        input.color.map(|v| tags::color.eq(v)),
        tags::updated_at.eq(Utc::now()),
        tags::updated_by.eq(user_id),
    );

    diesel::update(tags::table.filter(tags::id.eq(id)))
        .set(common)
        .returning(Tag::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to update tag")
}

pub async fn delete_tag(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(tags::table.filter(tags::id.eq(id)))
        .execute(db)
        .await
        .api_context("Failed to delete tag")?;

    if affected == 0 {
        return Err(ApiError::not_found("Tag not found"));
    }

    Ok(())
}

pub async fn list_tags(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListTagsQuery,
) -> Result<ResourceList<TagView>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> tags::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = tags::table.into_boxed();

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
        .get_result::<i64>(db)
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
        .load::<Tag>(db)
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
            .load::<(i64, i64)>(db)
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

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
