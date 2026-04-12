use std::collections::{HashMap, HashSet};

use crate::domain::metadata_types::DataType;
use crate::schema::{document_metadatas, document_types_metadata_types, documents, metadata_types};
use crate::shared::util::{ApiError, diesel_to_http};

use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDocumentMetadata {
    pub metadata_type_id: i64,
    pub value: String,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableDocumentMetadata {
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

pub async fn document_metadatas_upsert(
    user_id: i64,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    input: Vec<NewDocumentMetadata>,
) -> Result<(), ApiError> {
    // Validate the input metadata against the document type's rules,
    // including required fields, data types, and lookup choices.
    validate_document_metadata_input(db, document_id, &input).await?;

    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let rows: Vec<InsertableDocumentMetadata> = input
        .into_iter()
        .map(|m| InsertableDocumentMetadata {
            document_id,
            metadata_type_id: m.metadata_type_id,
            value: m.value,
            created_by: user_id,
            updated_by: user_id,
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
        .execute(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to save document_metadata"))?;

    Ok(())
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
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
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

    // Build a map of metadata type ID to validation rules, by joining from the document to its document type,
    // then to the allowed metadata types, and finally selecting the relevant fields from the metadata types.
    let rules: HashMap<i64, MetadataValidationRule> =
        documents::table
            .filter(documents::id.eq(document_id))
            .inner_join(document_types_metadata_types::table.on(
                document_types_metadata_types::document_type_id.eq(documents::document_type_id),
            ))
            .inner_join(
                metadata_types::table
                    .on(metadata_types::id.eq(document_types_metadata_types::metadata_type_id)),
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
