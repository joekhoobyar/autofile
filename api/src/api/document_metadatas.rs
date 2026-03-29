use std::sync::Arc;

use crate::AppState;
use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::schema::{document_metadatas};
use crate::domain::document_metadatas::DocumentMetadata;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, ApiError};

use axum::extract::State;
use serde::Deserialize;

use axum::{
    Router,
    routing::get,
    Json,
    extract::Path,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel::upsert::excluded;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentMetadata {
    metadata_type_id: i64,
    value: String
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableDocumentMetadata {
    document_id: i64,
    metadata_type_id: i64,
    value: String,
    created_by: i64,
    updated_by: i64,
}

pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<DocumentMetadata>, ApiError> {
    let row = document_metadatas::table
        .find((document_type_id, metadata_type_id))
        .select(DocumentMetadata::as_select())
        .first::<DocumentMetadata>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_metadata"))?;

    Ok(Json(row))
}


async fn upsert(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(document_id): Path<i64>,
    Json(input): Json<Vec<NewDocumentMetadata>>,
) -> Result<Json<Vec<DocumentMetadata>>, ApiError> {

    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let rows: Vec<InsertableDocumentMetadata> = input.into_iter().map(|m| InsertableDocumentMetadata {
        document_id,
        metadata_type_id: m.metadata_type_id,
        value: m.value,
        created_by: user.user_id,
        updated_by: user.user_id,
    }).collect();

    // Bulk upsert with Diesel.
    diesel::insert_into(document_metadatas::table)
        .values(&rows)
        .on_conflict((document_metadatas::document_id, document_metadatas::metadata_type_id))
        .do_update()
        .set((
            document_metadatas::value.eq(excluded(document_metadatas::value)),
            document_metadatas::updated_by.eq(excluded(document_metadatas::updated_by)),
            document_metadatas::updated_at.eq(diesel::dsl::now),
        ))
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to save document_metadata"))?;

    // Enqueue jobs to update document indexes for this document, as the metadata may be used in index rules.
    enqueue_document_index_document_updates(document_id, state).await?;

    // Fetch and return the updated rows.
    let rows = do_list(DbConn(db), document_id)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_metadatas"))?;

    Ok(Json(rows))
}


pub async fn do_list(
    DbConn(mut db): DbConn,
    document_id: i64,
) -> Result<Vec<DocumentMetadata>, diesel::result::Error> {
    return document_metadatas::table
        .filter(document_metadatas::document_id.eq(document_id))
        .select(DocumentMetadata::as_select())
        .order(document_metadatas::metadata_type_id.asc())
        .load::<DocumentMetadata>(&mut db)
        .await
}


pub async fn list(
    _user: AuthUser,
    DbConn(db): DbConn,
    Path(document_id): Path<i64>,
) -> Result<Json<Vec<DocumentMetadata>>, ApiError> {
    let rows = do_list(DbConn(db), document_id)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_metadatas"))?;

    Ok(Json(rows))
}

async fn delete_junction(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((document_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
            document_metadatas::table
                .filter(document_metadatas::document_id.eq(document_id))
                .filter(document_metadatas::metadata_type_id.eq(metadata_type_id))
        )
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete document_metadatas"))?;

    if affected == 0 {
        return Err(ApiError::not_found("document_metadatas not found"));
    }

    // Enqueue jobs to update document indexes for this document, as the metadata may be used in index rules.
    enqueue_document_index_document_updates(document_id, state).await?;

    Ok(Json(()))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_id}/metadata", get(list).post(upsert))
        .route("/{document_id}/metadata/{metadata_type_id}",
            get(get_by_ids).delete(delete_junction))
}
