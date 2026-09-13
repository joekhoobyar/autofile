use std::collections::HashMap;

use axum::http::StatusCode;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::domain::cabinets::{Cabinet, CabinetView};
use crate::schema::{cabinet_documents, cabinets};
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;
use crate::shared::serde::de_present_option;
use crate::shared::slugs::validate_slug;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewCabinet {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CabinetChangeset {
    pub name: Option<String>,
    pub description: Option<String>,

    /// Set a new parent, or null to detach. Omitted when unchanged.
    #[serde(default, deserialize_with = "de_present_option")]
    #[schema(value_type = Option<i64>)]
    pub parent_id: Option<Option<i64>>,
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

pub async fn get_cabinet(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<Cabinet, ApiError> {
    cabinets::table
        .find(id)
        .select(Cabinet::as_select())
        .first::<Cabinet>(db)
        .await
        .api_context("Failed to fetch cabinet")
}

pub async fn get_cabinet_by_slug(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    slug: String,
) -> Result<Cabinet, ApiError> {
    cabinets::table
        .filter(cabinets::slug.eq(slug))
        .select(Cabinet::as_select())
        .first::<Cabinet>(db)
        .await
        .api_context("Failed to fetch cabinet")
}

pub async fn create_cabinet(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: NewCabinet,
) -> Result<Cabinet, ApiError> {
    validate_slug(&input.slug)?;

    if let Some(parent_id) = input.parent_id
        && parent_id <= 0
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid parent cabinet",
        ));
    }

    diesel::insert_into(cabinets::table)
        .values((
            &input,
            cabinets::created_by.eq(user_id),
            cabinets::updated_by.eq(user_id),
        ))
        .returning(Cabinet::as_returning())
        .get_result(db)
        .await
        .api_context("Failed to create cabinet")
}

pub async fn update_cabinet(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    id: i64,
    input: CabinetChangeset,
) -> Result<Cabinet, ApiError> {
    let common = (
        input.name.map(|v| cabinets::name.eq(v)),
        input.description.map(|v| cabinets::description.eq(v)),
        cabinets::updated_at.eq(diesel::dsl::now),
        cabinets::updated_by.eq(user_id),
    );

    let base = diesel::update(cabinets::table.filter(cabinets::id.eq(id)));

    let updated = match input.parent_id {
        None => {
            base.set(common)
                .returning(Cabinet::as_returning())
                .get_result(db)
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
                .get_result(db)
                .await
        }
        Some(None) => {
            base.set((common, cabinets::parent_id.eq::<Option<i64>>(None)))
                .returning(Cabinet::as_returning())
                .get_result(db)
                .await
        }
    };

    updated.api_context("Failed to update cabinet")
}

pub async fn delete_cabinet(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(cabinets::table.filter(cabinets::id.eq(id)))
        .execute(db)
        .await
        .api_context("Failed to delete cabinet")?;

    if affected == 0 {
        return Err(ApiError::not_found("Cabinet not found"));
    }

    Ok(())
}

pub async fn list_cabinets(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListCabinetsQuery,
) -> Result<ResourceList<CabinetView>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> cabinets::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = cabinets::table.into_boxed();

        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                cabinets::slug
                    .ilike(pattern.clone())
                    .or(cabinets::name.ilike(pattern.clone()))
                    .or(cabinets::description.ilike(pattern)),
            )
        }

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
        .get_result::<i64>(db)
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
        .load::<Cabinet>(db)
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
            .load::<(i64, i64)>(db)
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

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
