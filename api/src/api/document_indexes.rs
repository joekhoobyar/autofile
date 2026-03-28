use std::sync::Arc;

use crate::AppState;
use crate::schema::{
    document_index_documents,
    document_index_templates,
    document_index_values,
    document_indexes,
};
use crate::domain::document_indexes::DocumentIndex;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, ApiError, ResourceList};

use serde::Deserialize;

use axum::{
    Router,
    routing::get,
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_indexes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentIndex {
    slug: String,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = document_indexes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentIndexChangeset {
    name: Option<String>,
    description: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIndexSortField {
    Id,
    Slug,
    Name,
    Description,
    Enabled,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentIndexesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // optional sort field
    pub sf: Option<DocumentIndexSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let row = document_indexes::table
        .find(id)
        .select(DocumentIndex::as_select())
        .first::<DocumentIndex>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_index"))?;

    Ok(Json(row))
}

pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let row = document_indexes::table
        .filter(document_indexes::slug.eq(slug))
        .select(DocumentIndex::as_select())
        .first::<DocumentIndex>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_index"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentIndex>,
) -> Result<Json<DocumentIndex>, ApiError> {
    let inserted: DocumentIndex = diesel::insert_into(document_indexes::table)
        .values((
            &input,
            document_indexes::created_by.eq(user.user_id),
            document_indexes::updated_by.eq(user.user_id),
        ))
        .returning(DocumentIndex::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create document_index"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentIndexChangeset>,
) -> Result<Json<DocumentIndex>, ApiError> {

    // Update + return the updated row in one round-trip.
    let updated: DocumentIndex =
        diesel::update(document_indexes::table.filter(document_indexes::id.eq(id)))
            .set((
                &input,
                document_indexes::updated_at.eq(diesel::dsl::now),
                document_indexes::updated_by.eq(user.user_id),
            )) // also update the timestamp
            .returning(DocumentIndex::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update document_index"))?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let document_index_id = id;

    db.transaction::<_, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {

            // Delete associated document index documents
            let value_ids = document_index_values::table
                .inner_join(
                    document_index_templates::table.on(
                        document_index_values::document_index_template_id
                            .eq(document_index_templates::id),
                    ),
                )
                .filter(document_index_templates::document_index_id.eq(document_index_id))
                .select(document_index_values::id);
            diesel::delete(
                document_index_documents::table.filter(
                    document_index_documents::document_index_value_id.eq_any(value_ids),
                ),
            )
            .execute(conn)
            .await?;

            // Delete associated document index values
            let template_ids = document_index_templates::table
                .filter(document_index_templates::document_index_id.eq(document_index_id))
                .select(document_index_templates::id);
            diesel::delete(
                document_index_values::table.filter(
                    document_index_values::document_index_template_id.eq_any(template_ids),
                ),
            )
            .execute(conn)
            .await?;

            // Delete associated document templates 
            diesel::delete(
                document_index_templates::table.filter(
                    document_index_templates::document_index_id.eq(document_index_id),
                ),
            )
            .execute(conn)
            .await?;

            // Delete the document index
            let affected = diesel::delete(document_indexes::table.filter(document_indexes::id.eq(document_index_id)))
                .execute(conn)
                .await?;

            if affected == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| {
        if matches!(e, diesel::result::Error::NotFound) {
            ApiError::not_found("Document type not found")
        } else {
            ApiError::new(diesel_to_http(e), "Failed to delete document_index")
        }
    })?;

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentIndexesQuery>,
) -> Result<Json<ResourceList<DocumentIndex>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> document_indexes::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let query = document_indexes::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query.filter(
                document_indexes::slug.ilike(pattern.clone())
                    .or(document_indexes::name.ilike(pattern.clone()))
                    .or(document_indexes::description.ilike(pattern)),
            )
        } else {
            query
        }
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count document_indexes"))?;

    let mut query: document_indexes::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentIndexSortField::Slug), Some(true)) =>
            query.order((document_indexes::slug.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Slug), _) =>
            query.order((document_indexes::slug.asc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Name), Some(true)) =>
            query.order((document_indexes::name.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Name), _) =>
            query.order((document_indexes::name.asc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Enabled), Some(true)) =>
            query.order((document_indexes::enabled.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Enabled), _) =>
            query.order((document_indexes::enabled.asc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Description), Some(true)) =>
            query.order((document_indexes::description.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::Description), _) =>
            query.order((document_indexes::description.asc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::CreatedAt), Some(true)) =>
            query.order((document_indexes::created_at.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::CreatedAt), _) =>
            query.order((document_indexes::created_at.asc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::UpdatedAt), Some(true)) =>
            query.order((document_indexes::updated_at.desc(), document_indexes::id.asc())), // tie-breaker
        (Some(DocumentIndexSortField::UpdatedAt), _) =>
            query.order((document_indexes::updated_at.asc(), document_indexes::id.asc())), // tie-breaker

        (Some(DocumentIndexSortField::Id), Some(true)) =>
            query.order(document_indexes::id.desc()),
        _ =>
            query.order(document_indexes::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(DocumentIndex::as_select())
        .load::<DocumentIndex>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_indexes"))?;

    Ok(Json(ResourceList { total, page, per_page, items }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/by-slug/{slug}", get(get_by_slug))
}
