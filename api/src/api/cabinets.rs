use std::{collections::HashMap, sync::Arc};

use crate::domain::cabinets::{Cabinet, CabinetView};
use crate::schema::{cabinet_documents, cabinets};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;
use crate::shared::serde::de_present_option;
use crate::shared::slugs::validate_slug;

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
    http::StatusCode,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewCabinet {
    slug: String,
    name: String,
    description: Option<String>,
    parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CabinetChangeset {
    name: Option<String>,
    description: Option<String>,

    /// Set a new parent, or null to detach. Omitted when unchanged.
    #[serde(default, deserialize_with = "de_present_option")]
    #[schema(value_type = Option<i64>)]
    parent_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCabinetsQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive search of slug, name, and description.
    pub q: Option<String>,
    /// Filter by parent: "null" for top-level cabinets, or a numeric ID.
    pub parent_id: Option<String>,
    /// Sort field.
    pub sf: Option<CabinetSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    responses(
        (status = 200, description = "Cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
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
        .api_context("Failed to fetch cabinet")?;

    Ok(Json(row))
}

#[utoipa::path(
    get,
    path = "/by-slug/{slug}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("slug" = String, Path, description = "Exact cabinet slug")),
    responses(
        (status = 200, description = "Cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
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
        .api_context("Failed to fetch cabinet")?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "cabinets",
    security(("bearer" = [])),
    request_body = NewCabinet,
    responses(
        (status = 200, description = "Created cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 409, description = "Slug is already taken", body = ApiError),
        (status = 422, description = "Invalid slug or parent cabinet", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewCabinet>,
) -> Result<Json<Cabinet>, ApiError> {
    validate_slug(&input.slug)?;

    if let Some(parent_id) = input.parent_id
        && parent_id <= 0
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid parent cabinet",
        ));
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
        .api_context("Failed to create cabinet")?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    request_body = CabinetChangeset,
    responses(
        (status = 200, description = "Updated cabinet", body = Cabinet),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
        (status = 422, description = "Invalid parent cabinet", body = ApiError),
    )
)]
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
        }
        Some(Some(parent_id)) => {
            if parent_id <= 0 || parent_id == id {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Invalid parent cabinet",
                ));
            }
            base.set((common, cabinets::parent_id.eq(parent_id)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        }
        Some(None) => {
            base.set((common, cabinets::parent_id.eq::<Option<i64>>(None)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        }
    };

    // Update + return the updated row in one round-trip.
    let updated: Cabinet = base.api_context("Failed to update cabinet")?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "cabinets",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Cabinet ID")),
    responses(
        (status = 200, description = "Cabinet deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Cabinet not found", body = ApiError),
    )
)]
async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(cabinets::table.filter(cabinets::id.eq(id)))
        .execute(&mut db)
        .await
        .api_context("Failed to delete cabinet")?;

    if affected == 0 {
        return Err(ApiError::not_found("Cabinet not found"));
    }

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "cabinets",
    security(("bearer" = [])),
    params(ListCabinetsQuery),
    responses(
        (status = 200, description = "Paginated cabinet list with document counts", body = ResourceList<CabinetView>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListCabinetsQuery>,
) -> Result<Json<ResourceList<CabinetView>>, ApiError> {
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
                cabinets::slug
                    .ilike(pattern.clone())
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
        .api_context("Failed to count cabinets")?;

    let mut query: cabinets::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(CabinetSortField::Slug), Some(true)) => {
            query.order((cabinets::slug.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Slug), _) => {
            query.order((cabinets::slug.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Name), Some(true)) => {
            query.order((cabinets::name.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Name), _) => {
            query.order((cabinets::name.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Description), Some(true)) => {
            query.order((cabinets::description.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Description), _) => {
            query.order((cabinets::description.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::ParentId), Some(true)) => {
            query.order((cabinets::parent_id.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::ParentId), _) => {
            query.order((cabinets::parent_id.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::CreatedAt), Some(true)) => {
            query.order((cabinets::created_at.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::CreatedAt), _) => {
            query.order((cabinets::created_at.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::UpdatedAt), Some(true)) => {
            query.order((cabinets::updated_at.desc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::UpdatedAt), _) => {
            query.order((cabinets::updated_at.asc(), cabinets::id.asc()))
        }
        (Some(CabinetSortField::Id), Some(true)) => query.order(cabinets::id.desc()),
        _ => query.order(cabinets::id.asc()),
    };

    let cabinets = query
        .limit(per_page)
        .offset(offset)
        .select(Cabinet::as_select())
        .load::<Cabinet>(&mut db)
        .await
        .api_context("Failed to list cabinets")?;
    let cabinet_ids: Vec<i64> = cabinets.iter().map(|cabinet| cabinet.id).collect();

    let mut document_counts_by_cabinet: HashMap<i64, i64> = HashMap::new();
    if !cabinet_ids.is_empty() {
        let document_count_rows: Vec<(i64, i64)> = cabinet_documents::table
            .filter(cabinet_documents::cabinet_id.eq_any(&cabinet_ids))
            .group_by(cabinet_documents::cabinet_id)
            .select((
                cabinet_documents::cabinet_id,
                diesel::dsl::count(cabinet_documents::document_id),
            ))
            .load::<(i64, i64)>(&mut db)
            .await
            .api_context("Failed to count cabinet documents")?;

        for (cabinet_id, document_count) in document_count_rows {
            document_counts_by_cabinet.insert(cabinet_id, document_count);
        }
    }

    let items = cabinets
        .into_iter()
        .map(|cabinet| CabinetView {
            id: cabinet.id,
            slug: cabinet.slug,
            name: cabinet.name,
            created_by: cabinet.created_by,
            created_at: cabinet.created_at,
            updated_by: cabinet.updated_by,
            updated_at: cabinet.updated_at,
            description: cabinet.description,
            parent_id: cabinet.parent_id,
            document_count: document_counts_by_cabinet
                .get(&cabinet.id)
                .cloned()
                .unwrap_or(0),
        })
        .collect();

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
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(get_by_slug))
}
