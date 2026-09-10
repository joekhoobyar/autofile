use std::{collections::HashMap, sync::Arc};

use crate::domain::document_types::{DocumentType, DocumentTypeView};
use crate::schema::{document_types, document_types_metadata_types, documents};
use crate::shared::app_state::AppState;
use crate::shared::auth::{AdminUser, AuthUser};
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http, validate_slug};

use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentType {
    slug: String,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentTypeChangeset {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentTypeSortField {
    Id,
    Slug,
    Name,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // optional sort field
    pub sf: Option<DocumentTypeSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentType>, ApiError> {
    let row = document_types::table
        .find(id)
        .select(DocumentType::as_select())
        .first::<DocumentType>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_type"))?;

    Ok(Json(row))
}

pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<DocumentType>, ApiError> {
    let row = document_types::table
        .filter(document_types::slug.eq(slug))
        .select(DocumentType::as_select())
        .first::<DocumentType>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_type"))?;

    Ok(Json(row))
}

async fn create(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentType>,
) -> Result<Json<DocumentType>, ApiError> {
    validate_slug(&input.slug)?;

    let inserted: DocumentType = diesel::insert_into(document_types::table)
        .values((
            &input,
            document_types::created_by.eq(user.user_id),
            document_types::updated_by.eq(user.user_id),
        ))
        .returning(DocumentType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create document_type"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentTypeChangeset>,
) -> Result<Json<DocumentType>, ApiError> {
    // Update + return the updated row in one round-trip.
    let updated: DocumentType =
        diesel::update(document_types::table.filter(document_types::id.eq(id)))
            .set((
                &input,
                document_types::updated_at.eq(diesel::dsl::now),
                document_types::updated_by.eq(user.user_id),
            )) // also update the timestamp
            .returning(DocumentType::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update document_type"))?;

    Ok(Json(updated))
}

async fn delete(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let document_type_id = id;

    if id == 1 {
        return Err(ApiError::bad_request("Cannot delete default document type"));
    }

    db.transaction::<_, diesel::result::Error, _>(async move |conn| {
        // Delete the join table records
        diesel::delete(
            document_types_metadata_types::table
                .filter(document_types_metadata_types::document_type_id.eq(document_type_id)),
        )
        .execute(conn)
        .await?;

        // Update the documents
        diesel::update(documents::table.filter(documents::document_type_id.eq(document_type_id)))
            .set((
                documents::document_type_id.eq(1),
                documents::updated_by.eq(user.user_id),
                documents::updated_at.eq(diesel::dsl::now),
            ))
            .execute(conn)
            .await?;

        // Delete the document type
        let affected =
            diesel::delete(document_types::table.filter(document_types::id.eq(document_type_id)))
                .execute(conn)
                .await?;

        if affected == 0 {
            return Err(diesel::result::Error::NotFound);
        }

        Ok(())
    })
    .await
    .map_err(|e| {
        if matches!(e, diesel::result::Error::NotFound) {
            ApiError::not_found("Document type not found")
        } else {
            ApiError::new(diesel_to_http(e), "Failed to delete document_type")
        }
    })?;

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentTypesQuery>,
) -> Result<Json<ResourceList<DocumentTypeView>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> document_types::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let query = document_types::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query.filter(
                document_types::slug
                    .ilike(pattern.clone())
                    .or(document_types::name.ilike(pattern.clone()))
                    .or(document_types::description.ilike(pattern)),
            )
        } else {
            query
        }
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count document_types"))?;

    let mut query: document_types::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentTypeSortField::Slug), Some(true)) => {
            query.order((document_types::slug.desc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::Slug), _) => {
            query.order((document_types::slug.asc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::Name), Some(true)) => {
            query.order((document_types::name.desc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::Name), _) => {
            query.order((document_types::name.asc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::Description), Some(true)) => {
            query.order((document_types::description.desc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::Description), _) => {
            query.order((document_types::description.asc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::CreatedAt), Some(true)) => {
            query.order((document_types::created_at.desc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::CreatedAt), _) => {
            query.order((document_types::created_at.asc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::UpdatedAt), Some(true)) => {
            query.order((document_types::updated_at.desc(), document_types::id.asc()))
        } // tie-breaker
        (Some(DocumentTypeSortField::UpdatedAt), _) => {
            query.order((document_types::updated_at.asc(), document_types::id.asc()))
        } // tie-breaker

        (Some(DocumentTypeSortField::Id), Some(true)) => query.order(document_types::id.desc()),
        _ => query.order(document_types::id.asc()),
    };

    let document_types = query
        .limit(per_page)
        .offset(offset)
        .select(DocumentType::as_select())
        .load::<DocumentType>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_types"))?;
    let document_type_ids: Vec<i64> = document_types
        .iter()
        .map(|document_type| document_type.id)
        .collect();

    let mut document_counts_by_type: HashMap<i64, i64> = HashMap::new();
    if !document_type_ids.is_empty() {
        let document_count_rows: Vec<(i64, i64)> = documents::table
            .filter(documents::document_type_id.eq_any(&document_type_ids))
            .group_by(documents::document_type_id)
            .select((
                documents::document_type_id,
                diesel::dsl::count(documents::id),
            ))
            .load::<(i64, i64)>(&mut db)
            .await
            .map_err(|e| {
                ApiError::new(diesel_to_http(e), "Failed to count document_type documents")
            })?;

        for (document_type_id, document_count) in document_count_rows {
            document_counts_by_type.insert(document_type_id, document_count);
        }
    }

    let items = document_types
        .into_iter()
        .map(|document_type| DocumentTypeView {
            id: document_type.id,
            slug: document_type.slug,
            name: document_type.name,
            created_by: document_type.created_by,
            created_at: document_type.created_at,
            updated_by: document_type.updated_by,
            updated_at: document_type.updated_at,
            description: document_type.description,
            document_count: document_counts_by_type
                .get(&document_type.id)
                .cloned()
                .unwrap_or(0),
        })
        .collect();

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
