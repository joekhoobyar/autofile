use bb8::PooledConnection;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::metadata_types::{DataType, MetadataType};
use crate::schema::{document_metadatas, document_types_metadata_types, metadata_types};
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;
use crate::shared::slugs::validate_slug;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMetadataType {
    pub slug: String,
    pub name: String,
    pub data_type: DataType,
    pub description: Option<String>,
    pub options: Option<Value>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MetadataTypeChangeset {
    pub name: Option<String>,
    pub data_type: Option<DataType>,
    pub description: Option<String>,
    pub options: Option<Value>,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetadataTypeSortField {
    Id,
    Slug,
    Name,
    DataType,
    Description,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListMetadataTypesQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive search of slug, name, data type, and description.
    pub q: Option<String>,
    /// Sort field.
    pub sf: Option<MetadataTypeSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListMetadataTypeValuesQuery {
    /// Optional case-insensitive substring filter.
    pub q: Option<String>,
    /// Maximum number of values (default 20, max 50).
    pub limit: Option<i64>,
}

pub async fn get_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<MetadataType, ApiError> {
    metadata_types::table
        .find(id)
        .select(MetadataType::as_select())
        .first::<MetadataType>(db)
        .await
        .api_context("Failed to fetch metadata_type")
}

pub async fn get_metadata_type_by_slug(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    slug: String,
) -> Result<MetadataType, ApiError> {
    metadata_types::table
        .filter(metadata_types::slug.eq(slug))
        .select(MetadataType::as_select())
        .first::<MetadataType>(db)
        .await
        .api_context("Failed to fetch metadata_type")
}

pub async fn list_metadata_type_values(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
    params: ListMetadataTypeValuesQuery,
) -> Result<Vec<String>, ApiError> {
    let data_type = metadata_types::table
        .filter(metadata_types::id.eq(id))
        .select(metadata_types::data_type)
        .first::<DataType>(db)
        .await
        .api_context("Failed to fetch metadata_type")?;

    if data_type != DataType::String {
        return Err(ApiError::unprocessable_entity(
            "Metadata value suggestions are only available for string metadata types",
        ));
    }

    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    let mut query = document_metadatas::table
        .filter(document_metadatas::metadata_type_id.eq(id))
        .filter(diesel::dsl::sql::<Bool>("btrim(value) <> ''"))
        .into_boxed();

    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        query = query.filter(document_metadatas::value.ilike(format!("%{}%", q)));
    }

    query
        .select(document_metadatas::value)
        .distinct()
        .order(document_metadatas::value.asc())
        .limit(limit)
        .load::<String>(db)
        .await
        .api_context("Failed to list metadata_type values")
}

pub async fn create_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: NewMetadataType,
) -> Result<MetadataType, ApiError> {
    validate_slug(&input.slug)?;

    diesel::insert_into(metadata_types::table)
        .values((
            &input,
            metadata_types::created_by.eq(user_id),
            metadata_types::updated_by.eq(user_id),
        ))
        .returning(MetadataType::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to create metadata_type")
}

pub async fn update_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    id: i64,
    input: MetadataTypeChangeset,
) -> Result<MetadataType, ApiError> {
    // Update + return the updated row in one round-trip.
    diesel::update(metadata_types::table.filter(metadata_types::id.eq(id)))
        .set((
            &input,
            metadata_types::updated_by.eq(user_id),
            metadata_types::updated_at.eq(diesel::dsl::now),
        ))
        .returning(MetadataType::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to update metadata_type")
}

pub async fn delete_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    let in_use: bool = diesel::select(exists(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::metadata_type_id.eq(id)),
    ))
    .get_result(db)
    .await
    .api_context("Failed to check metadata_type usage")?;

    if in_use {
        return Err(ApiError::conflict(
            "Metadata type is in use by a document type and cannot be deleted",
        ));
    }

    let has_document_metadata: bool = diesel::select(exists(
        document_metadatas::table.filter(document_metadatas::metadata_type_id.eq(id)),
    ))
    .get_result(db)
    .await
    .map_err(|e| {
        ApiError::from_diesel("Failed to check metadata_type document metadata usage", e)
    })?;

    if has_document_metadata {
        return Err(ApiError::conflict(
            "Metadata type is in use by document metadata and cannot be deleted",
        ));
    }

    let affected = diesel::delete(metadata_types::table.filter(metadata_types::id.eq(id)))
        .execute(db)
        .await
        .api_context("Failed to delete metadata_type")?;

    if affected == 0 {
        return Err(ApiError::not_found("Metadata type not found"));
    }

    Ok(())
}

pub async fn list_metadata_types(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListMetadataTypesQuery,
) -> Result<ResourceList<MetadataType>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> metadata_types::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let query = metadata_types::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/data_type/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query.filter(
                metadata_types::slug
                    .ilike(pattern.clone())
                    .or(metadata_types::name.ilike(pattern.clone()))
                    .or(metadata_types::data_type.ilike(pattern.clone()))
                    .or(metadata_types::description.ilike(pattern)),
            )
        } else {
            query
        }
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(db)
        .await
        .api_context("Failed to count metadata_types")?;

    let mut query: metadata_types::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(MetadataTypeSortField::Slug), Some(true)) => {
            query.order((metadata_types::slug.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Slug), _) => {
            query.order((metadata_types::slug.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Name), Some(true)) => {
            query.order((metadata_types::name.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Name), _) => {
            query.order((metadata_types::name.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::DataType), Some(true)) => {
            query.order((metadata_types::data_type.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::DataType), _) => {
            query.order((metadata_types::data_type.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Description), Some(true)) => {
            query.order((metadata_types::description.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Description), _) => {
            query.order((metadata_types::description.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::CreatedAt), Some(true)) => {
            query.order((metadata_types::created_at.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::CreatedAt), _) => {
            query.order((metadata_types::created_at.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::UpdatedAt), Some(true)) => {
            query.order((metadata_types::updated_at.desc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::UpdatedAt), _) => {
            query.order((metadata_types::updated_at.asc(), metadata_types::id.asc()))
        }
        (Some(MetadataTypeSortField::Id), Some(true)) => query.order(metadata_types::id.desc()),
        _ => query.order(metadata_types::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(MetadataType::as_select())
        .load::<MetadataType>(db)
        .await
        .api_context("Failed to list metadata_types")?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
