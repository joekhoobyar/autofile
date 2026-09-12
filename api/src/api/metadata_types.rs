use std::sync::Arc;

use crate::domain::metadata_types::{DataType, MetadataType};
use crate::schema::{document_metadatas, document_types_metadata_types, metadata_types};
use crate::shared::app_state::AppState;
use crate::shared::auth::{AdminUser, AuthUser};
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;
use crate::shared::slugs::validate_slug;

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::sql_types::Bool;
use diesel_async::RunQueryDsl;
use serde_json::Value;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewMetadataType {
    slug: String,
    name: String,
    data_type: DataType,
    description: Option<String>,
    options: Option<Value>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct MetadataTypeChangeset {
    name: Option<String>,
    data_type: Option<DataType>,
    description: Option<String>,
    options: Option<Value>,
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

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    responses(
        (status = 200, description = "Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<MetadataType>, ApiError> {
    let row = metadata_types::table
        .find(id)
        .select(MetadataType::as_select())
        .first::<MetadataType>(&mut db)
        .await
        .api_context("Failed to fetch metadata_type")?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact metadata type slug")),
    responses(
        (status = 200, description = "Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<MetadataType>, ApiError> {
    let row = metadata_types::table
        .filter(metadata_types::slug.eq(slug))
        .select(MetadataType::as_select())
        .first::<MetadataType>(&mut db)
        .await
        .api_context("Failed to fetch metadata_type")?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/{id}/values",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(
        ("id" = i64, Path, description = "Metadata Type ID"),
        ListMetadataTypeValuesQuery,
    ),
    responses(
        (status = 200, description = "Distinct stored values for string metadata types", body = Vec<String>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
        (status = 422, description = "Not a string metadata type", body = ApiError),
    )
)]
pub async fn list_values(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Query(params): Query<ListMetadataTypeValuesQuery>,
) -> Result<Json<Vec<String>>, ApiError> {
    let data_type = metadata_types::table
        .filter(metadata_types::id.eq(id))
        .select(metadata_types::data_type)
        .first::<DataType>(&mut db)
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

    let values = query
        .select(document_metadatas::value)
        .distinct()
        .order(document_metadatas::value.asc())
        .limit(limit)
        .load::<String>(&mut db)
        .await
        .api_context("Failed to list metadata_type values")?;

    Ok(Json(values))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "metadata-types",
    security(("bearer" = [])),
    request_body = NewMetadataType,
    responses(
        (status = 200, description = "Created Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug", body = ApiError),
    )
)]
async fn create(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewMetadataType>,
) -> Result<Json<MetadataType>, ApiError> {
    validate_slug(&input.slug)?;

    let inserted: MetadataType = diesel::insert_into(metadata_types::table)
        .values((
            &input,
            metadata_types::created_by.eq(user.user_id),
            metadata_types::updated_by.eq(user.user_id),
        ))
        .returning(MetadataType::as_returning())
        .get_result(&mut db)
        .await
        .api_context("Failed to create metadata_type")?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    request_body = MetadataTypeChangeset,
    responses(
        (status = 200, description = "Updated Metadata Type", body = MetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
    )
)]
async fn update(
    user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<MetadataTypeChangeset>,
) -> Result<Json<MetadataType>, ApiError> {
    // Update + return the updated row in one round-trip.
    let updated: MetadataType =
        diesel::update(metadata_types::table.filter(metadata_types::id.eq(id)))
            .set((
                &input,
                metadata_types::updated_by.eq(user.user_id),
                metadata_types::updated_at.eq(diesel::dsl::now),
            ))
            .returning(MetadataType::as_returning())
            .get_result(&mut db)
            .await
            .api_context("Failed to update metadata_type")?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Metadata Type ID")),
    responses(
        (status = 200, description = "Metadata Type deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Metadata Type not found", body = ApiError),
        (status = 409, description = "Metadata Type is still in use", body = ApiError),
    )
)]
async fn delete(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let in_use: bool = diesel::select(exists(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::metadata_type_id.eq(id)),
    ))
    .get_result(&mut db)
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
    .get_result(&mut db)
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
        .execute(&mut db)
        .await
        .api_context("Failed to delete metadata_type")?;

    if affected == 0 {
        return Err(ApiError::not_found("Metadata type not found"));
    }

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "metadata-types",
    security(("bearer" = [])),
    params(ListMetadataTypesQuery),
    responses(
        (status = 200, description = "Paginated Metadata Type list", body = ResourceList<MetadataType>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListMetadataTypesQuery>,
) -> Result<Json<ResourceList<MetadataType>>, ApiError> {
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
        .get_result::<i64>(&mut db)
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
        .load::<MetadataType>(&mut db)
        .await
        .api_context("Failed to list metadata_types")?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(list_values))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
