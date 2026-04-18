use std::sync::Arc;

use crate::domain::metadata_types::{DataType, MetadataType};
use crate::schema::{document_types_metadata_types, metadata_types};
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
use diesel_async::{AsyncConnection, RunQueryDsl};
use serde_json::Value;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewMetadataType {
    slug: String,
    name: String,
    data_type: DataType,
    description: Option<String>,
    options: Option<Value>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct MetadataTypeChangeset {
    name: Option<String>,
    data_type: Option<DataType>,
    description: Option<String>,
    options: Option<Value>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct ListMetadataTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // optional sort field
    pub sf: Option<MetadataTypeSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

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
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch metadata_type"))?;

    Ok(Json(row))
}

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
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch metadata_type"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewMetadataType>,
) -> Result<Json<MetadataType>, ApiError> {
    let inserted: MetadataType = diesel::insert_into(metadata_types::table)
        .values((
            &input,
            metadata_types::created_by.eq(user.user_id),
            metadata_types::updated_by.eq(user.user_id),
        ))
        .returning(MetadataType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create metadata_type"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
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
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update metadata_type"))?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    db.transaction::<_, diesel::result::Error, _>(move |conn| {
        Box::pin(async move {
            // Delete the join table records
            diesel::delete(
                document_types_metadata_types::table
                    .filter(document_types_metadata_types::metadata_type_id.eq(id)),
            )
            .execute(conn)
            .await?;

            // Delete the metadata type
            let affected = diesel::delete(metadata_types::table.filter(metadata_types::id.eq(id)))
                .execute(conn)
                .await?;

            if affected == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            Ok(())
        })
    })
    .await
    .map_err(|e| {
        if matches!(e, diesel::result::Error::NotFound) {
            ApiError::not_found("Metadata type not found")
        } else {
            ApiError::new(diesel_to_http(e), "Failed to delete metadata_type")
        }
    })?;

    Ok(Json(()))
}

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
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count metadata_types"))?;

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
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list metadata_types"))?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/by-slug/{slug}", get(get_by_slug))
}
