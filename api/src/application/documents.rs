use std::collections::{HashMap};

use crate::schema::{cabinet_documents, document_metadatas, documents, metadata_types, tag_documents};
use crate::domain::documents::{Document, DocumentView};
use crate::shared::util::{diesel_to_http, ApiError};

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

/**
 * This function retrieves a document by its ID and constructs a DocumentView, which includes the document's metadata, associated cabinets, and tags.
 */
pub async fn get_document_view(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<DocumentView, ApiError> {
    let document = documents::table
        .find(id)
        .select(Document::as_select())
        .first::<Document>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document"))?;

    let metadata_rows: Vec<(String, String)> = document_metadatas::table
        .inner_join(metadata_types::table)
        .filter(document_metadatas::document_id.eq(document.id))
        .select((metadata_types::slug, document_metadatas::value))
        .load::<(String, String)>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document metadata"))?;
    let metadata: HashMap<String, String> = metadata_rows.into_iter().collect();

    let cabinet_rows: Vec<i64> = cabinet_documents::table
        .filter(cabinet_documents::document_id.eq(document.id))
        .select(cabinet_documents::cabinet_id)
        .load::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list cabinets for document"))?;
    let cabinet_ids: Vec<i64> = cabinet_rows.into_iter().collect();

    let tag_rows: Vec<i64> = tag_documents::table
        .filter(tag_documents::document_id.eq(document.id))
        .select(tag_documents::tag_id)
        .load::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list tags for document"))?;
    let tag_ids: Vec<i64> = tag_rows.into_iter().collect();

    Ok(DocumentView {
        id: document.id,
        title: document.title,
        document_type_id: document.document_type_id,
        cabinet_ids,
        tag_ids,
        metadata,
        created_by: document.created_by,
        created_at: document.created_at,
        updated_by: document.updated_by,
        updated_at: document.updated_at,
    })
}
