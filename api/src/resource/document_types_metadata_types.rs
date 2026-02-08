use std::sync::Arc;

use crate::AppState;
use crate::auth::AuthUser;
use crate::schema::{document_types_metadata_types, metadata_types};
use crate::util::{diesel_to_http, ApiError};
use crate::extractors::DbConn;
use crate::resource::document_types::DocumentType;
use crate::resource::metadata_types::MetadataType;

use serde::{Deserialize, Serialize};

use axum::{
    Router,
    routing::get,
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Serialize, Identifiable, Associations, Queryable, Selectable)]
#[diesel(belongs_to(DocumentType))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(primary_key(document_type_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentTypeMetadataTypeChangeset {
    required: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentTypesMetadataTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    pub document_type_id: Option<i64>,
    pub metadata_type_id: Option<i64>,
}

pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let row = document_types_metadata_types::table
        .find((document_type_id, metadata_type_id))
        .select(DocumentTypeMetadataType::as_select())
        .first::<DocumentTypeMetadataType>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_type_metadata_type"))?;

    Ok(Json(row))
}

async fn create(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentTypeMetadataType>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let inserted: DocumentTypeMetadataType = diesel::insert_into(document_types_metadata_types::table)
        .values(&input)
        .returning(DocumentTypeMetadataType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create document_type_metadata_type"))?;

    Ok(Json(inserted))
}

async fn update(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
    Json(input): Json<DocumentTypeMetadataTypeChangeset>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let changes = input;

    // Update + return the updated row in one round-trip.
    let updated: DocumentTypeMetadataType =
        diesel::update(
                document_types_metadata_types::table
                    .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
                    .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id))
            )
            .set(&changes)
            .returning(DocumentTypeMetadataType::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update document_type_metadata_type"))?;

    Ok(Json(updated))
}

async fn delete_junction(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
            document_types_metadata_types::table
                .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
                .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id))
        )
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete document_type_metadata_type"))?;

    if affected == 0 {
        return Err(ApiError::not_found("document_type_metadata_type not found"));
    }

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentTypesMetadataTypesQuery>,
) -> Result<Json<Vec<DocumentTypeMetadataType>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = document_types_metadata_types::table.inner_join(metadata_types::table).into_boxed();

    // Optional search: case-insensitive substring on slug/name/data_type/description
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query
            .filter(
                metadata_types::slug.ilike(pattern.clone())
                    .or(metadata_types::name.ilike(pattern.clone()))
                    .or(metadata_types::data_type.ilike(pattern.clone()))
                    .or(metadata_types::description.ilike(pattern)),
            );
    }

    // Filter by document type or metadata type
    if let Some(id) = params.document_type_id {
        query = query.filter(document_types_metadata_types::document_type_id.eq(id));
    }
    if let Some(id) = params.metadata_type_id {
        query = query.filter(document_types_metadata_types::metadata_type_id.eq(id));
    }

    let rows = query
        .order((document_types_metadata_types::document_type_id.desc(), document_types_metadata_types::metadata_type_id.desc()))
        .limit(per_page)
        .offset(offset)
        .select(DocumentTypeMetadataType::as_select())
        .load::<DocumentTypeMetadataType>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_types_metadata_types"))?;

    Ok(Json(rows))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{document_type_id}/{metadata_type_id}",
            get(get_by_ids).patch(update).delete(delete_junction))
}
