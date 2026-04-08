use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::AppState;
use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::document_metadatas::DocumentMetadata;
use crate::domain::metadata_types::DataType;
use crate::schema::{document_metadatas, document_types_metadata_types, documents, metadata_types};
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, diesel_to_http};

use axum::extract::State;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;

use axum::{Json, Router, extract::Path, routing::get};
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl, pooled_connection::bb8::PooledConnection};

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentMetadata {
    metadata_type_id: i64,
    value: String,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableDocumentMetadata {
    document_id: i64,
    metadata_type_id: i64,
    value: String,
    created_by: i64,
    updated_by: i64,
}

#[derive(Debug)]
struct MetadataValidationRule {
    required: bool,
    data_type: DataType,
    options: Option<Value>,
}

fn extract_lookup_choices(options: Option<&Value>) -> Result<HashSet<&str>, ApiError> {
    let Some(options) = options else {
        return Ok(HashSet::new());
    };

    let Some(choices) = options.get("choices") else {
        return Ok(HashSet::new());
    };

    let choices = choices.as_array().ok_or_else(|| {
        ApiError::internal_server_error("Lookup metadata type options.choices must be an array")
    })?;

    let mut result = HashSet::with_capacity(choices.len());
    for choice in choices {
        let choice = choice.as_str().ok_or_else(|| {
            ApiError::internal_server_error(
                "Lookup metadata type options.choices must contain only strings",
            )
        })?;
        result.insert(choice);
    }

    Ok(result)
}

fn validate_metadata_value(
    metadata_type_id: i64,
    rule: &MetadataValidationRule,
    value: &str,
) -> Result<(), ApiError> {
    let trimmed = value.trim();

    if rule.required && trimmed.is_empty() {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} is required for this document type and cannot be empty",
            metadata_type_id
        )));
    }

    if trimmed.is_empty() {
        return Ok(());
    }

    match rule.data_type {
        DataType::String => Ok(()),
        DataType::Date => NaiveDate::parse_from_str(trimmed, "%Y-%m-%d")
            .map(|_| ())
            .map_err(|_| {
                ApiError::unprocessable_entity(&format!(
                    "Metadata field {} must be a valid date in YYYY-MM-DD format",
                    metadata_type_id
                ))
            }),
        DataType::Lookup => {
            let choices = extract_lookup_choices(rule.options.as_ref())?;
            if choices.contains(trimmed) {
                Ok(())
            } else {
                Err(ApiError::unprocessable_entity(&format!(
                    "Metadata field {} must be one of the configured choices",
                    metadata_type_id
                )))
            }
        }
    }
}

async fn validate_document_metadata_input(
    db: &mut PooledConnection<'static, AsyncPgConnection>,
    document_id: i64,
    input: &[NewDocumentMetadata],
) -> Result<(), ApiError> {
    let metadata_type_ids: Vec<i64> = input
        .iter()
        .map(|m| m.metadata_type_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if metadata_type_ids.is_empty() {
        return Ok(());
    }

    let rules: HashMap<i64, MetadataValidationRule> = documents::table
        .filter(documents::id.eq(document_id))
        .inner_join(document_types_metadata_types::table.on(
            document_types_metadata_types::document_type_id.eq(documents::document_type_id),
        ))
        .inner_join(
            metadata_types::table.on(
                metadata_types::id.eq(document_types_metadata_types::metadata_type_id),
            ),
        )
        .filter(document_types_metadata_types::metadata_type_id.eq_any(&metadata_type_ids))
        .select((
            document_types_metadata_types::metadata_type_id,
            document_types_metadata_types::required,
            metadata_types::data_type,
            metadata_types::options,
        ))
        .load::<(i64, bool, DataType, Option<Value>)>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to validate document metadata"))?
        .into_iter()
        .map(|(metadata_type_id, required, data_type, options)| {
            (
                metadata_type_id,
                MetadataValidationRule {
                    required,
                    data_type,
                    options,
                },
            )
        })
        .collect();

    for m in input {
        let Some(rule) = rules.get(&m.metadata_type_id) else {
            return Err(ApiError::unprocessable_entity(&format!(
                "Metadata field {} is not allowed for this document type",
                m.metadata_type_id
            )));
        };
        validate_metadata_value(m.metadata_type_id, rule, &m.value)?;
    }

    Ok(())
}

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

    // Validate the input metadata against the document type's rules,
    // including required fields, data types, and lookup choices.
    validate_document_metadata_input(&mut db, document_id, &input).await?;

    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let rows: Vec<InsertableDocumentMetadata> = input
        .into_iter()
        .map(|m| InsertableDocumentMetadata {
            document_id,
            metadata_type_id: m.metadata_type_id,
            value: m.value,
            created_by: user.user_id,
            updated_by: user.user_id,
        })
        .collect();

    // Bulk upsert with Diesel.
    diesel::insert_into(document_metadatas::table)
        .values(&rows)
        .on_conflict((
            document_metadatas::document_id,
            document_metadatas::metadata_type_id,
        ))
        .do_update()
        .set((
            document_metadatas::value.eq(excluded(document_metadatas::value)),
            document_metadatas::updated_by.eq(excluded(document_metadatas::updated_by)),
            document_metadatas::updated_at.eq(diesel::dsl::now),
        ))
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to save document_metadata"))?;

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
