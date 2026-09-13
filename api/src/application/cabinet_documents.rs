use std::sync::Arc;

use bb8::PooledConnection;
use chrono::Utc;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::cabinet_documents::CabinetDocument;
use crate::schema::cabinet_documents;
use crate::shared::app_state::AppState;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = cabinet_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewCabinetDocument {
    pub document_id: i64,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = cabinet_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableCabinetDocument {
    pub cabinet_id: i64,
    pub document_id: i64,
    pub updated_by: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CabinetDocumentSortField {
    DocumentId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCabinetDocumentsQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Sort field.
    pub sf: Option<CabinetDocumentSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

pub async fn get_cabinet_document(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    cabinet_id: i64,
    document_id: i64,
) -> Result<CabinetDocument, ApiError> {
    cabinet_documents::table
        .find((cabinet_id, document_id))
        .select(CabinetDocument::as_select())
        .first::<CabinetDocument>(db)
        .await
        .api_context("Failed to fetch cabinet_document")
}

pub async fn upsert_cabinet_documents(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    cabinet_id: i64,
    input: Vec<NewCabinetDocument>,
) -> Result<Vec<CabinetDocument>, ApiError> {
    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let values: Vec<InsertableCabinetDocument> = input
        .into_iter()
        .map(|m| InsertableCabinetDocument {
            cabinet_id,
            document_id: m.document_id,
            updated_by: user_id,
        })
        .collect();

    // Bulk upsert with Diesel.
    let items = diesel::insert_into(cabinet_documents::table)
        .values(&values)
        .on_conflict((
            cabinet_documents::cabinet_id,
            cabinet_documents::document_id,
        ))
        .do_update()
        .set((
            cabinet_documents::updated_by.eq(excluded(cabinet_documents::updated_by)),
            cabinet_documents::updated_at.eq(Utc::now()),
        ))
        .returning(CabinetDocument::as_returning())
        .get_results(db)
        .await
        .api_context("Failed to save cabinet_document")?;

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    for doc in values {
        enqueue_document_index_document_updates(doc.document_id, state.clone()).await?;
    }

    Ok(items)
}

pub async fn list_cabinet_documents(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListCabinetDocumentsQuery,
) -> Result<ResourceList<CabinetDocument>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total = cabinet_documents::table
        .count()
        .get_result::<i64>(db)
        .await
        .api_context("Failed to count cabinet_documents")?;

    let mut query = cabinet_documents::table.into_boxed();
    query = match (params.sf, params.sd) {
        (Some(CabinetDocumentSortField::UpdatedAt), Some(true)) => query.order((
            cabinet_documents::updated_at.desc(),
            cabinet_documents::document_id.asc(),
        )),
        (Some(CabinetDocumentSortField::UpdatedAt), _) => query.order((
            cabinet_documents::updated_at.asc(),
            cabinet_documents::document_id.asc(),
        )),
        (Some(CabinetDocumentSortField::DocumentId), Some(true)) => {
            query.order(cabinet_documents::document_id.desc())
        }
        _ => query.order(cabinet_documents::document_id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(CabinetDocument::as_select())
        .load::<CabinetDocument>(db)
        .await
        .api_context("Failed to list cabinet_documents")?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}

pub async fn delete_cabinet_documents(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    cabinet_id: i64,
    document_ids: Vec<i64>,
) -> Result<(), ApiError> {
    diesel::delete(
        cabinet_documents::table
            .filter(cabinet_documents::cabinet_id.eq(cabinet_id))
            .filter(cabinet_documents::document_id.eq_any(&document_ids)),
    )
    .execute(db)
    .await
    .api_context("Failed to delete cabinet_documents")?;

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    for document_id in document_ids {
        enqueue_document_index_document_updates(document_id, state.clone()).await?;
    }

    Ok(())
}

pub async fn delete_cabinet_document(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    cabinet_id: i64,
    document_id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(
        cabinet_documents::table
            .filter(cabinet_documents::cabinet_id.eq(cabinet_id))
            .filter(cabinet_documents::document_id.eq(document_id)),
    )
    .execute(db)
    .await
    .api_context("Failed to delete cabinet_document")?;

    if affected == 0 {
        return Err(ApiError::not_found("cabinet_document not found"));
    }

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    enqueue_document_index_document_updates(document_id, state).await?;

    Ok(())
}
