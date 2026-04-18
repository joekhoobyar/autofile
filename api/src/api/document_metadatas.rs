use std::sync::Arc;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::application::document_metadatas::{NewDocumentMetadata, document_metadatas_upsert};
use crate::domain::document_metadatas::DocumentMetadata;
use crate::schema::{document_metadatas, document_types_metadata_types, documents};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::extract::State;

use axum::{Json, Router, extract::Path, routing::get};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

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
    document_metadatas_upsert(user.user_id, &mut db, document_id, input).await?;

    // Validate the input metadata against the document type's rules,
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
        .await;
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
    match documents::table
        .filter(documents::id.eq(document_id))
        .inner_join(
            document_types_metadata_types::table.on(
                document_types_metadata_types::document_type_id.eq(documents::document_type_id),
            ),
        )
        .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id))
        .filter(document_types_metadata_types::required.eq(true))
        .select(documents::id)
        .first::<i64>(&mut db)
        .await
    {
        Ok(_) => {
            return Err(ApiError::conflict(
                "Metadata field is required for this document type and cannot be deleted",
            ));
        }
        Err(diesel::result::Error::NotFound) => {}
        Err(e) => {
            return Err(ApiError::new(
                diesel_to_http(e),
                "Failed to validate document_metadata deletion",
            ));
        }
    }

    let affected = diesel::delete(
        document_metadatas::table
            .filter(document_metadatas::document_id.eq(document_id))
            .filter(document_metadatas::metadata_type_id.eq(metadata_type_id)),
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
        .route(
            "/{document_id}/metadata/{metadata_type_id}",
            get(get_by_ids).delete(delete_junction),
        )
}
