use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::domain::document_metadatas::DocumentMetadata;
use crate::domain::metadata_types::DataType;
use crate::schema::{document_metadatas, document_types_metadata_types, documents, metadata_types};
use crate::shared::app_state::AppState;
use crate::shared::errors::{ApiError, ApiErrorContext};

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::Value;

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel::upsert::excluded;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct NewDocumentMetadata {
    pub metadata_type_id: i64,
    /// String or lookup metadata value to store.
    #[schema(example = "Acme Corporation")]
    pub string_value: Option<String>,
    /// Numeric metadata value to store. Number metadata types are not supported yet.
    #[schema(value_type = Option<f64>, example = 123.45)]
    pub number_value: Option<BigDecimal>,
    /// Date metadata value to store. Clients must send YYYY-MM-DD regardless of the configured frontend display format.
    #[schema(example = "2026-08-25")]
    pub date_value: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_metadatas)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableDocumentMetadata {
    document_id: i64,
    metadata_type_id: i64,
    created_by: i64,
    updated_by: i64,
    string_value: Option<String>,
    number_value: Option<BigDecimal>,
    date_value: Option<NaiveDate>,
}

#[derive(Debug)]
struct NormalizedDocumentMetadata {
    metadata_type_id: i64,
    string_value: Option<String>,
    number_value: Option<BigDecimal>,
    date_value: Option<NaiveDate>,
}

impl NormalizedDocumentMetadata {
    fn is_empty(&self) -> bool {
        self.string_value.is_none() && self.number_value.is_none() && self.date_value.is_none()
    }
}

pub(crate) fn typed_metadata_value_to_string(
    string_value: Option<String>,
    number_value: Option<BigDecimal>,
    date_value: Option<NaiveDate>,
) -> Option<String> {
    string_value
        .or_else(|| number_value.map(|value| value.to_string()))
        .or_else(|| date_value.map(|value| value.to_string()))
}

pub async fn get_document_metadata(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    metadata_type_id: i64,
) -> Result<DocumentMetadata, ApiError> {
    document_metadatas::table
        .find((document_id, metadata_type_id))
        .select(DocumentMetadata::as_select())
        .first::<DocumentMetadata>(db)
        .await
        .api_context("Failed to fetch document_metadata")
}

pub async fn list_document_metadatas(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
) -> Result<Vec<DocumentMetadata>, ApiError> {
    document_metadatas::table
        .filter(document_metadatas::document_id.eq(document_id))
        .select(DocumentMetadata::as_select())
        .order(document_metadatas::metadata_type_id.asc())
        .load::<DocumentMetadata>(db)
        .await
        .api_context("Failed to list document_metadatas")
}

pub async fn upsert_document_metadatas(
    state: Arc<AppState>,
    user_id: i64,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    input: Vec<NewDocumentMetadata>,
) -> Result<Vec<DocumentMetadata>, ApiError> {
    document_metadatas_upsert(user_id, db, document_id, input).await?;
    enqueue_document_index_document_updates(document_id, state).await?;
    list_document_metadatas(db, document_id).await
}

pub async fn delete_document_metadata(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    metadata_type_id: i64,
) -> Result<(), ApiError> {
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
        .first::<i64>(db)
        .await
    {
        Ok(_) => {
            return Err(ApiError::conflict(
                "Metadata field is required for this document type and cannot be deleted",
            ));
        }
        Err(diesel::result::Error::NotFound) => {}
        Err(e) => {
            return Err(ApiError::from_diesel(
                "Failed to validate document_metadata deletion",
                e,
            ));
        }
    }

    let affected = diesel::delete(
        document_metadatas::table
            .filter(document_metadatas::document_id.eq(document_id))
            .filter(document_metadatas::metadata_type_id.eq(metadata_type_id)),
    )
    .execute(db)
    .await
    .api_context("Failed to delete document_metadatas")?;

    if affected == 0 {
        return Err(ApiError::not_found("document_metadatas not found"));
    }

    enqueue_document_index_document_updates(document_id, state).await?;

    Ok(())
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
    const MAX_METADATA_UPSERT_ITEMS: usize = 1000;

    if input.len() > MAX_METADATA_UPSERT_ITEMS {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_REQUEST,
            "Too many metadata items in request",
        ));
    }

    // Validate the input metadata against the document type's rules,
    // including required fields, data types, and lookup choices.
    // Required blank values fail validation here, so any blank value
    // remaining afterwards is optional and means "delete the stored row".
    let normalized_input = validate_document_metadata_input(db, document_id, &input).await?;

    // Partition the input into blank optional values (to delete) and
    // non-blank values (to upsert).
    let mut delete_ids: Vec<i64> = Vec::new();
    let mut upsert_input: Vec<NormalizedDocumentMetadata> =
        Vec::with_capacity(normalized_input.len().min(MAX_METADATA_UPSERT_ITEMS));
    for m in normalized_input {
        if m.is_empty() {
            if !delete_ids.contains(&m.metadata_type_id) {
                delete_ids.push(m.metadata_type_id);
            }
        } else {
            upsert_input.push(m);
        }
    }

    if !delete_ids.is_empty() {
        diesel::delete(
            document_metadatas::table
                .filter(document_metadatas::document_id.eq(document_id))
                .filter(document_metadatas::metadata_type_id.eq_any(&delete_ids)),
        )
        .execute(&mut *db)
        .await
        .api_context("Failed to delete document_metadata")?;
    }

    if upsert_input.is_empty() {
        return Ok(());
    }

    // Prepare the rows to upsert, setting created_by and updated_by to the current user.
    // It is worth allocating memory so that we can bulk upsert with Diesel, rather than doing individual queries in a loop.
    let rows: Vec<InsertableDocumentMetadata> = upsert_input
        .into_iter()
        .map(|m| InsertableDocumentMetadata {
            document_id,
            metadata_type_id: m.metadata_type_id,
            created_by: user_id,
            updated_by: user_id,
            string_value: m.string_value,
            number_value: m.number_value,
            date_value: m.date_value,
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
            document_metadatas::string_value.eq(excluded(document_metadatas::string_value)),
            document_metadatas::number_value.eq(excluded(document_metadatas::number_value)),
            document_metadatas::date_value.eq(excluded(document_metadatas::date_value)),
            document_metadatas::updated_by.eq(excluded(document_metadatas::updated_by)),
            document_metadatas::updated_at.eq(diesel::dsl::now),
        ))
        .execute(db)
        .await
        .api_context("Failed to save document_metadata")?;

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

fn empty_normalized(metadata_type_id: i64) -> NormalizedDocumentMetadata {
    NormalizedDocumentMetadata {
        metadata_type_id,
        string_value: None,
        number_value: None,
        date_value: None,
    }
}

fn validate_empty_metadata_value(
    metadata_type_id: i64,
    rule: &MetadataValidationRule,
) -> Result<NormalizedDocumentMetadata, ApiError> {
    if rule.required {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} is required for this document type and cannot be empty",
            metadata_type_id
        )));
    }

    Ok(empty_normalized(metadata_type_id))
}

fn normalize_string_metadata_value(
    metadata_type_id: i64,
    rule: &MetadataValidationRule,
    value: &str,
) -> Result<NormalizedDocumentMetadata, ApiError> {
    let trimmed = value.trim();

    if rule.required && trimmed.is_empty() {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} is required for this document type and cannot be empty",
            metadata_type_id
        )));
    }

    if trimmed.is_empty() {
        return Ok(empty_normalized(metadata_type_id));
    }

    match rule.data_type {
        DataType::String => Ok(NormalizedDocumentMetadata {
            metadata_type_id,
            string_value: Some(value.to_string()),
            number_value: None,
            date_value: None,
        }),
        DataType::Date => Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} must use date_value",
            metadata_type_id
        ))),
        DataType::Lookup => {
            let choices = extract_lookup_choices(rule.options.as_ref())?;
            if choices.contains(trimmed) {
                Ok(NormalizedDocumentMetadata {
                    metadata_type_id,
                    string_value: Some(value.to_string()),
                    number_value: None,
                    date_value: None,
                })
            } else {
                Err(ApiError::unprocessable_entity(&format!(
                    "Metadata field {} must be one of the configured choices",
                    metadata_type_id
                )))
            }
        }
    }
}

fn normalize_date_metadata_value(
    metadata_type_id: i64,
    rule: &MetadataValidationRule,
    value: &str,
) -> Result<NormalizedDocumentMetadata, ApiError> {
    let trimmed = value.trim();

    if rule.required && trimmed.is_empty() {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} is required for this document type and cannot be empty",
            metadata_type_id
        )));
    }

    if trimmed.is_empty() {
        return Ok(empty_normalized(metadata_type_id));
    }

    if rule.data_type != DataType::Date {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} must use string_value",
            metadata_type_id
        )));
    }

    let date_value = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").map_err(|_| {
        ApiError::unprocessable_entity(&format!(
            "Metadata field {} must be a valid date in YYYY-MM-DD format",
            metadata_type_id
        ))
    })?;

    Ok(NormalizedDocumentMetadata {
        metadata_type_id,
        string_value: None,
        number_value: None,
        date_value: Some(date_value),
    })
}

fn normalize_metadata_value(
    metadata_type_id: i64,
    rule: &MetadataValidationRule,
    input: &NewDocumentMetadata,
) -> Result<NormalizedDocumentMetadata, ApiError> {
    let typed_value_count = [
        input.string_value.is_some(),
        input.number_value.is_some(),
        input.date_value.is_some(),
    ]
    .into_iter()
    .filter(|is_set| *is_set)
    .count();

    if typed_value_count > 1 {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} must set at most one typed value",
            metadata_type_id
        )));
    }

    if input.number_value.is_some() {
        return Err(ApiError::unprocessable_entity(&format!(
            "Metadata field {} cannot use number_value because number metadata types are not supported yet",
            metadata_type_id
        )));
    }

    if let Some(value) = input.string_value.as_deref() {
        return normalize_string_metadata_value(metadata_type_id, rule, value);
    }

    if let Some(value) = input.date_value.as_deref() {
        return normalize_date_metadata_value(metadata_type_id, rule, value);
    }

    validate_empty_metadata_value(metadata_type_id, rule)
}

async fn validate_document_metadata_input(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    input: &[NewDocumentMetadata],
) -> Result<Vec<NormalizedDocumentMetadata>, ApiError> {
    let metadata_type_ids: Vec<i64> = input
        .iter()
        .map(|m| m.metadata_type_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if metadata_type_ids.is_empty() {
        return Ok(Vec::new());
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
            .api_context("Failed to validate document metadata")?
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

    let mut normalized = Vec::with_capacity(input.len());
    for m in input {
        let Some(rule) = rules.get(&m.metadata_type_id) else {
            return Err(ApiError::unprocessable_entity(&format!(
                "Metadata field {} is not allowed for this document type",
                m.metadata_type_id
            )));
        };
        normalized.push(normalize_metadata_value(m.metadata_type_id, rule, m)?);
    }

    Ok(normalized)
}
