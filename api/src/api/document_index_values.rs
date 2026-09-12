use std::sync::Arc;

use crate::application::document_index_values::{
    count_document_index_value_documents, list_document_index_value_ancestors,
};
use crate::domain::document_indexes::{DocumentIndexValue, DocumentIndexValueView};
use crate::schema::document_index_values;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ApiErrorContext, ResourceList};

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIndexValueSortField {
    Id,
    Value,
    DocumentIndexTemplateId,
    ParentId,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentIndexValuesQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive value substring search.
    pub q: Option<String>,
    /// Filter by parent: "null" for top-level values, or a numeric ID.
    pub parent_id: Option<String>,
    /// Narrow to one template.
    pub document_index_template_id: Option<i64>,
    /// Sort field.
    pub sf: Option<DocumentIndexValueSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/values",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ListDocumentIndexValuesQuery,
    ),
    responses(
        (status = 200, description = "Paginated index value list with document counts", body = ResourceList<DocumentIndexValueView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Query(params): Query<ListDocumentIndexValuesQuery>,
) -> Result<Json<ResourceList<DocumentIndexValueView>>, ApiError> {
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
        .api_context("Failed to count document_index_values")?;

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

    let values = query
        .limit(per_page)
        .offset(offset)
        .select(DocumentIndexValue::as_select())
        .load::<DocumentIndexValue>(&mut db)
        .await
        .api_context("Failed to list document_index_values")?;

    let value_ids: Vec<i64> = values.iter().map(|value| value.id).collect();
    let document_counts =
        count_document_index_value_documents(&mut db, document_index_id, &value_ids).await?;
    let items = values
        .into_iter()
        .map(|value| DocumentIndexValueView {
            document_count: document_counts.get(&value.id).copied().unwrap_or(0),
            id: value.id,
            value: value.value,
            document_index_id: value.document_index_id,
            document_index_template_id: value.document_index_template_id,
            parent_id: value.parent_id,
            is_leaf: value.is_leaf,
        })
        .collect();

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/values/{id}/ancestors",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Index Value ID"),
    ),
    responses(
        (status = 200, description = "Ancestor values from the root down to the parent", body = Vec<DocumentIndexValue>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Index value not found", body = ApiError),
    )
)]
pub async fn ancestors(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<Vec<DocumentIndexValue>>, ApiError> {
    let items = list_document_index_value_ancestors(&mut db, document_index_id, id).await?;
    Ok(Json(items))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(ancestors))
}
