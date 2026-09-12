use std::sync::Arc;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::tag_documents::TagDocument;
use crate::schema::tag_documents;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

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
#[diesel(table_name = tag_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewTagDocument {
    document_id: i64,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = tag_documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableTagDocument {
    tag_id: i64,
    document_id: i64,
    updated_by: i64,
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

#[utoipa::path(
    get,
    path = "/{tag_id}/documents/{document_id}",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
        ("document_id" = i64, Path, description = "Document ID"),
    ),
    responses(
        (status = 200, description = "Tag-document association", body = TagDocument),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((tag_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<TagDocument>, ApiError> {
    let row = tag_documents::table
        .find((tag_id, document_id))
        .select(TagDocument::as_select())
        .first::<TagDocument>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch tag_document"))?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{tag_id}/documents",
    tag = "tags",
    security(("bearer" = [])),
    params(("tag_id" = i64, Path, description = "Tag ID")),
    request_body = Vec<NewTagDocument>,
    responses(
        (status = 200, description = "Created or updated associations", body = Vec<TagDocument>),
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
    Path(tag_id): Path<i64>,
    Json(input): Json<Vec<NewTagDocument>>,
) -> Result<Json<Vec<TagDocument>>, ApiError> {
    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let values: Vec<InsertableTagDocument> = input
        .into_iter()
        .map(|m| InsertableTagDocument {
            tag_id,
            document_id: m.document_id,
            updated_by: user.user_id,
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
        .get_results(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to save tag_document"))?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    for doc in values {
        enqueue_document_index_document_updates(doc.document_id, state.clone()).await?;
    }

    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/{tag_id}/documents",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
        ListTagDocumentsQuery,
    ),
    responses(
        (status = 200, description = "Paginated tag-document associations", body = ResourceList<TagDocument>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListTagDocumentsQuery>,
) -> Result<Json<ResourceList<TagDocument>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let total = tag_documents::table
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count tag_documents"))?;

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
        .load::<TagDocument>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list tag_documents"))?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

#[utoipa::path(
    post,
    path = "/{tag_id}/documents/delete",
    tag = "tags",
    security(("bearer" = [])),
    params(("tag_id" = i64, Path, description = "Tag ID")),
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
    Path(tag_id): Path<i64>,
    Json(input): Json<Vec<i64>>,
) -> Result<Json<()>, ApiError> {
    diesel::delete(
        tag_documents::table
            .filter(tag_documents::tag_id.eq(tag_id))
            .filter(tag_documents::document_id.eq_any(&input)),
    )
    .execute(&mut db)
    .await
    .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete tag_documents"))?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    for document_id in input {
        enqueue_document_index_document_updates(document_id, state.clone()).await?;
    }

    Ok(Json(()))
}

#[utoipa::path(
    delete,
    path = "/{tag_id}/documents/{document_id}",
    tag = "tags",
    security(("bearer" = [])),
    params(
        ("tag_id" = i64, Path, description = "Tag ID"),
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
    Path((tag_id, document_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
        tag_documents::table
            .filter(tag_documents::tag_id.eq(tag_id))
            .filter(tag_documents::document_id.eq(document_id)),
    )
    .execute(&mut db)
    .await
    .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete tag_document"))?;

    if affected == 0 {
        return Err(ApiError::not_found("tag_document not found"));
    }

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
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
