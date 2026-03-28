use std::sync::Arc;

use crate::AppState;
use crate::schema::cabinets;
use crate::domain::cabinets::Cabinet;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http, de_present_option};

use serde::Deserialize;

use axum::{
    Router,
    routing::get,
    Json,
    http::StatusCode,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewCabinet {
    slug: String,
    name: String,
    description: Option<String>,
    parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CabinetChangeset {
    name: Option<String>,
    description: Option<String>,

    #[serde(default, deserialize_with = "de_present_option")]
    parent_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CabinetSortField {
    Id,
    Slug,
    Name,
    Description,
    ParentId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListCabinetsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // Filter by parent_id: "null" for null, or numeric value
    pub parent_id: Option<String>,
    // optional sort field
    pub sf: Option<CabinetSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Cabinet>, ApiError> {
    let row = cabinets::table
        .find(id)
        .select(Cabinet::as_select())
        .first::<Cabinet>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch cabinet"))?;

    Ok(Json(row))
}

pub async fn get_by_slug(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(slug): Path<String>,
) -> Result<Json<Cabinet>, ApiError> {
    let row = cabinets::table
        .filter(cabinets::slug.eq(slug))
        .select(Cabinet::as_select())
        .first::<Cabinet>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch cabinet"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewCabinet>,
) -> Result<Json<Cabinet>, ApiError> {
    if let Some(parent_id) = input.parent_id {
        if parent_id <= 0 {
            return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Invalid parent cabinet"));
        }
    }

    let inserted: Cabinet = diesel::insert_into(cabinets::table)
        .values((
            &input,
            cabinets::created_by.eq(user.user_id),
            cabinets::updated_by.eq(user.user_id),
        ))
        .returning(Cabinet::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create cabinet"))?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<CabinetChangeset>,
) -> Result<Json<Cabinet>, ApiError> {
    let patch = input;

    // Common assignments (no parent_id here)
    let common = (
        patch.name.map(|v| cabinets::name.eq(v)),
        patch.description.map(|v| cabinets::description.eq(v)),
        cabinets::updated_at.eq(diesel::dsl::now),
        cabinets::updated_by.eq(user.user_id),
    );

    let base = diesel::update(cabinets::table.filter(cabinets::id.eq(id)));

    let base = match patch.parent_id {
        None => {
            base.set(common)
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
        Some(Some(parent_id)) => {
            if parent_id <= 0 || parent_id == id {
                return Err(ApiError::new(StatusCode::UNPROCESSABLE_ENTITY, "Invalid parent cabinet"));
            }
            base.set((common, cabinets::parent_id.eq(parent_id)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
        Some(None) => {
            base.set((common, cabinets::parent_id.eq::<Option<i64>>(None)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
    };

    // Update + return the updated row in one round-trip.
    let updated: Cabinet = base
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update cabinet"))?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(cabinets::table.filter(cabinets::id.eq(id)))
        .execute(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete cabinet"))?;

    if affected == 0 {
        return Err(ApiError::not_found("Cabinet not found"));
    }

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListCabinetsQuery>,
) -> Result<Json<ResourceList<Cabinet>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> cabinets::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let mut query = cabinets::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                cabinets::slug.ilike(pattern.clone())
                    .or(cabinets::name.ilike(pattern.clone()))
                    .or(cabinets::description.ilike(pattern)),
            )
        }

        // Optional filter by parent: "null" for null values, or numeric string
        if let Some(ref parent_id_str) = params.parent_id {
            if parent_id_str == "null" {
                query = query.filter(cabinets::parent_id.is_null());
            } else if let Ok(parent_id) = parent_id_str.parse::<i64>() {
                query = query.filter(cabinets::parent_id.eq(parent_id));
            }
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count cabinets"))?;

    let mut query: cabinets::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(CabinetSortField::Slug), Some(true)) =>
            query.order((cabinets::slug.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::Slug), _) =>
            query.order((cabinets::slug.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::Name), Some(true)) =>
            query.order((cabinets::name.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::Name), _) =>
            query.order((cabinets::name.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::Description), Some(true)) =>
            query.order((cabinets::description.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::Description), _) =>
            query.order((cabinets::description.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::ParentId), Some(true)) =>
            query.order((cabinets::parent_id.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::ParentId), _) =>
            query.order((cabinets::parent_id.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::CreatedAt), Some(true)) =>
            query.order((cabinets::created_at.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::CreatedAt), _) =>
            query.order((cabinets::created_at.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::UpdatedAt), Some(true)) =>
            query.order((cabinets::updated_at.desc(), cabinets::id.asc())),
        (Some(CabinetSortField::UpdatedAt), _) =>
            query.order((cabinets::updated_at.asc(), cabinets::id.asc())),
        (Some(CabinetSortField::Id), Some(true)) =>
            query.order(cabinets::id.desc()),
        _ =>
            query.order(cabinets::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(Cabinet::as_select())
        .load::<Cabinet>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list cabinets"))?;

    Ok(Json(ResourceList { total, page, per_page, items }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update).delete(delete))
        .route("/by-slug/{slug}", get(get_by_slug))
}
