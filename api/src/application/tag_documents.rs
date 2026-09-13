use std::sync::Arc;

use bb8::PooledConnection;
use chrono::Utc;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::tag_documents::TagDocument;
use crate::schema::tag_documents;
use crate::shared::app_state::AppState;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = tag_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewTagDocument {
    pub document_id: i64,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = tag_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableTagDocument {
    pub tag_id: i64,
    pub document_id: i64,
    pub updated_by: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagDocumentSortField {
    DocumentId,
    UpdatedAt,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListTagDocumentsQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Sort field.
    pub sf: Option<TagDocumentSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

pub async fn get_tag_document(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    tag_id: i64,
    document_id: i64,
) -> Result<TagDocument, ApiError> {
    tag_documents::table
        .find((tag_id, document_id))
        .select(TagDocument::as_select())
        .first::<TagDocument>(db)
        .await
        .api_context("Failed to fetch tag_document")
}

pub async fn upsert_tag_documents(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    tag_id: i64,
    input: Vec<NewTagDocument>,
) -> Result<Vec<TagDocument>, ApiError> {
    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let values: Vec<InsertableTagDocument> = input
        .into_iter()
        .map(|m| InsertableTagDocument {
            tag_id,
            document_id: m.document_id,
            updated_by: user_id,
        })
        .collect();

    // Bulk upsert with Diesel.
    let items = diesel::insert_into(tag_documents::table)
        .values(&values)
        .on_conflict((tag_documents::tag_id, tag_documents::document_id))
        .do_update()
        .set((
            tag_documents::updated_by.eq(excluded(tag_documents::updated_by)),
            tag_documents::updated_at.eq(Utc::now()),
        ))
        .returning(TagDocument::as_returning())
        .get_results(db)
        .await
        .api_context("Failed to save tag_document")?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    for doc in values {
        enqueue_document_index_document_updates(doc.document_id, state.clone()).await?;
    }

    Ok(items)
}

pub async fn list_tag_documents(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListTagDocumentsQuery,
) -> Result<ResourceList<TagDocument>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total = tag_documents::table
        .count()
        .get_result::<i64>(db)
        .await
        .api_context("Failed to count tag_documents")?;

    let mut query = tag_documents::table.into_boxed();
    query = match (params.sf, params.sd) {
        (Some(TagDocumentSortField::UpdatedAt), Some(true)) => query.order((
            tag_documents::updated_at.desc(),
            tag_documents::document_id.asc(),
        )),
        (Some(TagDocumentSortField::UpdatedAt), _) => query.order((
            tag_documents::updated_at.asc(),
            tag_documents::document_id.asc(),
        )),
        (Some(TagDocumentSortField::DocumentId), Some(true)) => {
            query.order(tag_documents::document_id.desc())
        }
        _ => query.order(tag_documents::document_id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(TagDocument::as_select())
        .load::<TagDocument>(db)
        .await
        .api_context("Failed to list tag_documents")?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}

pub async fn delete_tag_documents(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    tag_id: i64,
    document_ids: Vec<i64>,
) -> Result<(), ApiError> {
    diesel::delete(
        tag_documents::table
            .filter(tag_documents::tag_id.eq(tag_id))
            .filter(tag_documents::document_id.eq_any(&document_ids)),
    )
    .execute(db)
    .await
    .api_context("Failed to delete tag_documents")?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    for document_id in document_ids {
        enqueue_document_index_document_updates(document_id, state.clone()).await?;
    }

    Ok(())
}

pub async fn delete_tag_document(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    tag_id: i64,
    document_id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(
        tag_documents::table
            .filter(tag_documents::tag_id.eq(tag_id))
            .filter(tag_documents::document_id.eq(document_id)),
    )
    .execute(db)
    .await
    .api_context("Failed to delete tag_document")?;

    if affected == 0 {
        return Err(ApiError::not_found("tag_document not found"));
    }

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    enqueue_document_index_document_updates(document_id, state).await?;

    Ok(())
}
