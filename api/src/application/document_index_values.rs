use std::collections::HashMap;

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel::sql_types::{Array, BigInt, Bool, Nullable, Text};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::domain::document_indexes::{DocumentIndexValue, DocumentIndexValueView};
use crate::schema::document_index_values;
use crate::shared::errors::ApiError;
use crate::shared::responses::ResourceList;

#[derive(Debug, Clone, Copy, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIndexValueSortField {
    Id,
    Value,
    DocumentIndexTemplateId,
    ParentId,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
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

#[derive(Debug, QueryableByName)]
struct DocumentIndexValueRow {
    #[diesel(sql_type = BigInt)]
    id: i64,
    #[diesel(sql_type = Text)]
    value: String,
    #[diesel(sql_type = BigInt)]
    document_index_id: i64,
    #[diesel(sql_type = BigInt)]
    document_index_template_id: i64,
    #[diesel(sql_type = Nullable<BigInt>)]
    parent_id: Option<i64>,
    #[diesel(sql_type = Bool)]
    is_leaf: bool,
}

#[derive(Debug, QueryableByName)]
struct DocumentIndexValueDocumentCountRow {
    #[diesel(sql_type = BigInt)]
    root_id: i64,
    #[diesel(sql_type = BigInt)]
    document_count: i64,
}

pub async fn list_document_index_values(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    params: ListDocumentIndexValuesQuery,
) -> Result<ResourceList<DocumentIndexValueView>, ApiError> {
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
        .get_result::<i64>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to count document_index_values", e))?;

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
        .load::<DocumentIndexValue>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_index_values", e))?;

    let value_ids: Vec<i64> = values.iter().map(|value| value.id).collect();
    let document_counts =
        count_document_index_value_documents(db, document_index_id, &value_ids).await?;
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

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}

pub async fn count_document_index_value_documents(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    value_ids: &[i64],
) -> Result<HashMap<i64, i64>, ApiError> {
    if value_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows = diesel::sql_query(
        r#"
        WITH RECURSIVE descendants AS (
            SELECT v.id AS root_id, v.id AS value_id
            FROM document_index_values v
            WHERE v.id = ANY($1) AND v.document_index_id = $2

            UNION ALL

            SELECT d.root_id, child.id
            FROM descendants d
            JOIN document_index_values child ON child.parent_id = d.value_id
            WHERE child.document_index_id = $2
        )
        SELECT
            d.root_id,
            COUNT(DISTINCT did.document_id) AS document_count
        FROM descendants d
        LEFT JOIN document_index_documents did
            ON did.document_index_value_id = d.value_id
        GROUP BY d.root_id
        "#,
    )
    .bind::<Array<BigInt>, _>(value_ids)
    .bind::<BigInt, _>(document_index_id)
    .load::<DocumentIndexValueDocumentCountRow>(db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to count document_index_value documents", e))?;

    Ok(rows
        .into_iter()
        .map(|row| (row.root_id, row.document_count))
        .collect())
}

pub async fn list_document_index_value_ancestors(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    id: i64,
) -> Result<Vec<DocumentIndexValue>, ApiError> {
    let rows = diesel::sql_query(
        r#"
        WITH RECURSIVE nodes AS (
            SELECT t.id, t.parent_id, 0 AS depth
            FROM document_index_values t
            WHERE t.id = $1 AND t.document_index_id = $2

            UNION ALL

            SELECT p.id, p.parent_id, d.depth + 1
            FROM document_index_values p
            JOIN nodes d ON p.id = d.parent_id
            WHERE p.document_index_id = $2
        )
        SELECT v.*
        FROM document_index_values v
        JOIN nodes n ON v.id = n.id
        ORDER BY n.depth DESC
        "#,
    )
    .bind::<BigInt, _>(id)
    .bind::<BigInt, _>(document_index_id)
    .load::<DocumentIndexValueRow>(db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to fetch document_index_value ancestors", e))?;

    Ok(rows
        .into_iter()
        .map(|row| DocumentIndexValue {
            id: row.id,
            value: row.value,
            document_index_id: row.document_index_id,
            document_index_template_id: row.document_index_template_id,
            parent_id: row.parent_id,
            is_leaf: row.is_leaf,
        })
        .collect())
}
