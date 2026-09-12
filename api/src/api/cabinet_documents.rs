use std::sync::Arc;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::cabinet_documents::CabinetDocument;
use crate::schema::cabinet_documents;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;

use axum::extract::State;
use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

use chrono::Utc;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = cabinet_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewCabinetDocument {
    document_id: i64,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = cabinet_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableCabinetDocument {
    cabinet_id: i64,
    document_id: i64,
    updated_by: i64,
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

#[utoipa::path(
    get,
    path = "/{cabinet_id}/documents/{document_id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Cabinet-document association", body = CabinetDocument),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((cabinet_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<CabinetDocument>, ApiError> {
    let row = cabinet_documents::table
        .find((cabinet_id, document_id))
        .select(CabinetDocument::as_select())
        .first::<CabinetDocument>(&mut db)
        .await
        .api_context("Failed to fetch cabinet_document")?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{cabinet_id}/documents",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("cabinet_id" = i64, Path, description = "Cabinet ID")),
    request_body = Vec<NewCabinetDocument>,
    responses(
        (status = 200, description = "Created or updated associations", body = Vec<CabinetDocument>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 422, description = "Unprocessable request", body = ApiError),
    )
)]
async fn upsert(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(cabinet_id): Path<i64>,
    Json(input): Json<Vec<NewCabinetDocument>>,
) -> Result<Json<Vec<CabinetDocument>>, ApiError> {
    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let values: Vec<InsertableCabinetDocument> = input
        .into_iter()
        .map(|m| InsertableCabinetDocument {
            cabinet_id,
            document_id: m.document_id,
            updated_by: user.user_id,
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
        .get_results(&mut db)
        .await
        .api_context("Failed to save cabinet_document")?;

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    for doc in values {
        enqueue_document_index_document_updates(doc.document_id, state.clone()).await?;
    }

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/{cabinet_id}/documents",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ListCabinetDocumentsQuery,
    ),
    responses(
        (status = 200, description = "Paginated cabinet-document associations", body = ResourceList<CabinetDocument>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListCabinetDocumentsQuery>,
) -> Result<Json<ResourceList<CabinetDocument>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total = cabinet_documents::table
        .count()
        .get_result::<i64>(&mut db)
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
        .load::<CabinetDocument>(&mut db)
        .await
        .api_context("Failed to list cabinet_documents")?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

#[utoipa::path(
    post,
    path = "/{cabinet_id}/documents/delete",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("cabinet_id" = i64, Path, description = "Cabinet ID")),
    request_body = Vec<i64>,
    responses(
        (status = 200, description = "Associations removed"),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(cabinet_id): Path<i64>,
    Json(input): Json<Vec<i64>>,
) -> Result<Json<()>, ApiError> {
    diesel::delete(
        cabinet_documents::table
            .filter(cabinet_documents::cabinet_id.eq(cabinet_id))
            .filter(cabinet_documents::document_id.eq_any(&input)),
    )
    .execute(&mut db)
    .await
    .api_context("Failed to delete cabinet_documents")?;

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    for document_id in input {
        enqueue_document_index_document_updates(document_id, state.clone()).await?;
    }

    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/{cabinet_id}/documents/{document_id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(
        ("cabinet_id" = i64, Path, description = "Cabinet ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Association removed"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path((cabinet_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
        cabinet_documents::table
            .filter(cabinet_documents::cabinet_id.eq(cabinet_id))
            .filter(cabinet_documents::document_id.eq(document_id)),
    )
    .execute(&mut db)
    .await
    .api_context("Failed to delete cabinet_document")?;

    if affected == 0 {
        return Err(ApiError::not_found("cabinet_document not found"));
    }

    // Enqueue jobs to update document indexes for this document, as the cabinets may be used in index rules.
    enqueue_document_index_document_updates(document_id, state).await?;

    Ok(Json(()))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(upsert))
        .routes(routes!(delete))
        .routes(routes!(get_by_ids))
        .routes(routes!(delete_junction))
}
