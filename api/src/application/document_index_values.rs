use bb8::PooledConnection;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Bool, Nullable, Text};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::domain::document_indexes::DocumentIndexValue;
use crate::shared::util::{ApiError, diesel_to_http};

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
    .map_err(|e| {
        ApiError::new(
            diesel_to_http(e),
            "Failed to fetch document_index_value ancestors",
        )
    })?;

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
