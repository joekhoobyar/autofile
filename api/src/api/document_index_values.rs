use std::sync::Arc;

use crate::application::document_index_values::list_document_index_value_ancestors;
use crate::domain::document_indexes::DocumentIndexValue;
use crate::schema::document_index_values;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Path, Query},
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIndexValueSortField {
    Id,
    Value,
    DocumentIndexTemplateId,
    ParentId,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentIndexValuesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // Filter by parent_id: "null" for null, or numeric value
    pub parent_id: Option<String>,
    // Filter by document_index_template_id
    pub document_index_template_id: Option<i64>,
    // optional sort field
    pub sf: Option<DocumentIndexValueSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Query(params): Query<ListDocumentIndexValuesQuery>,
) -> Result<Json<ResourceList<DocumentIndexValue>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> document_index_values::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = document_index_values::table
            .filter(document_index_values::document_index_id.eq(document_index_id))
            .into_boxed();

        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(document_index_values::value.ilike(pattern));
        }

        if let Some(ref parent_id_str) = params.parent_id {
            if parent_id_str == "null" {
                query = query.filter(document_index_values::parent_id.is_null());
            } else if let Ok(parent_id) = parent_id_str.parse::<i64>() {
                query = query.filter(document_index_values::parent_id.eq(parent_id));
            }
        }

        if let Some(template_id) = params.document_index_template_id {
            query = query.filter(document_index_values::document_index_template_id.eq(template_id));
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count document_index_values"))?;

    let mut query: document_index_values::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentIndexValueSortField::Value), Some(true)) => query.order((
            document_index_values::value.desc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::Value), _) => query.order((
            document_index_values::value.asc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::DocumentIndexTemplateId), Some(true)) => query.order((
            document_index_values::document_index_template_id.desc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::DocumentIndexTemplateId), _) => query.order((
            document_index_values::document_index_template_id.asc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::ParentId), Some(true)) => query.order((
            document_index_values::parent_id.desc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::ParentId), _) => query.order((
            document_index_values::parent_id.asc(),
            document_index_values::id.asc(),
        )),
        (Some(DocumentIndexValueSortField::Id), Some(true)) => {
            query.order(document_index_values::id.desc())
        }
        (Some(DocumentIndexValueSortField::Id), _) => query.order(document_index_values::id.asc()),
        _ => query.order((
            document_index_values::value.asc(),
            document_index_values::id.asc(),
        )),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(DocumentIndexValue::as_select())
        .load::<DocumentIndexValue>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_index_values"))?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

pub async fn ancestors(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentIndexValue>>, ApiError> {
    let items = list_document_index_value_ancestors(&mut db, document_index_id, id).await?;
    Ok(Json(items))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_index_id}/values", get(list))
        .route("/{document_index_id}/values/{id}/ancestors", get(ancestors))
}
