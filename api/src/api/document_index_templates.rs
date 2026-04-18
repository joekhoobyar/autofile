use std::sync::Arc;

use crate::domain::document_indexes::DocumentIndexTemplate;
use crate::schema::document_index_templates;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{ApiError, ResourceList, de_present_option, diesel_to_http};

use serde::Deserialize;

use axum::{
    Json, Router,
    extract::{Path, Query},
    http::StatusCode,
    routing::get,
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentIndexTemplate {
    template: String,
    is_leaf: bool,
    enabled: bool,
    parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentIndexTemplateChangeset {
    template: Option<String>,
    is_leaf: Option<bool>,
    enabled: Option<bool>,

    #[serde(default, deserialize_with = "de_present_option")]
    parent_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentIndexTemplateSortField {
    Id,
    Template,
    IsLeaf,
    Enabled,
    ParentId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Deserialize)]
pub struct ListDocumentIndexTemplatesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // Filter by parent_id: "null" for null, or numeric value
    pub parent_id: Option<String>,
    // optional sort field
    pub sf: Option<DocumentIndexTemplateSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    let row = document_index_templates::table
        .filter(document_index_templates::document_index_id.eq(document_index_id))
        .filter(document_index_templates::id.eq(id))
        .select(DocumentIndexTemplate::as_select())
        .first::<DocumentIndexTemplate>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch document_index_template"))?;

    Ok(Json(row))
}

async fn create(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Json(input): Json<NewDocumentIndexTemplate>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    if document_index_id <= 0 {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid document index",
        ));
    }

    if let Some(parent_id) = input.parent_id {
        if parent_id <= 0 {
            return Err(ApiError::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Invalid parent template",
            ));
        }
    }

    let inserted: DocumentIndexTemplate = diesel::insert_into(document_index_templates::table)
        .values((
            &input,
            document_index_templates::document_index_id.eq(document_index_id),
            document_index_templates::created_by.eq(user.user_id),
            document_index_templates::updated_by.eq(user.user_id),
        ))
        .returning(DocumentIndexTemplate::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| {
            ApiError::new(
                diesel_to_http(e),
                "Failed to create document_index_template",
            )
        })?;

    Ok(Json(inserted))
}

async fn update(
    user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
    Json(input): Json<DocumentIndexTemplateChangeset>,
) -> Result<Json<DocumentIndexTemplate>, ApiError> {
    let patch = input;

    let common = (
        patch
            .template
            .map(|v| document_index_templates::template.eq(v)),
        patch
            .is_leaf
            .map(|v| document_index_templates::is_leaf.eq(v)),
        patch
            .enabled
            .map(|v| document_index_templates::enabled.eq(v)),
        document_index_templates::updated_at.eq(diesel::dsl::now),
        document_index_templates::updated_by.eq(user.user_id),
    );

    let base = diesel::update(
        document_index_templates::table
            .filter(document_index_templates::document_index_id.eq(document_index_id))
            .filter(document_index_templates::id.eq(id)),
    );

    let base = match patch.parent_id {
        None => {
            base.set(common)
                .returning(DocumentIndexTemplate::as_returning())
                .get_result(&mut db)
                .await
        }
        Some(Some(parent_id)) => {
            if parent_id <= 0 || parent_id == id {
                return Err(ApiError::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Invalid parent template",
                ));
            }
            base.set((common, document_index_templates::parent_id.eq(parent_id)))
                .returning(DocumentIndexTemplate::as_returning())
                .get_result(&mut db)
                .await
        }
        Some(None) => {
            base.set((
                common,
                document_index_templates::parent_id.eq::<Option<i64>>(None),
            ))
            .returning(DocumentIndexTemplate::as_returning())
            .get_result(&mut db)
            .await
        }
    };

    let updated: DocumentIndexTemplate = base.map_err(|e| {
        ApiError::new(
            diesel_to_http(e),
            "Failed to update document_index_template",
        )
    })?;

    Ok(Json(updated))
}

async fn delete(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_index_id, id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
        document_index_templates::table
            .filter(document_index_templates::document_index_id.eq(document_index_id))
            .filter(document_index_templates::id.eq(id)),
    )
    .execute(&mut db)
    .await
    .map_err(|e| {
        ApiError::new(
            diesel_to_http(e),
            "Failed to delete document_index_template",
        )
    })?;

    if affected == 0 {
        return Err(ApiError::not_found("Document index template not found"));
    }

    Ok(Json(()))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(document_index_id): Path<i64>,
    Query(params): Query<ListDocumentIndexTemplatesQuery>,
) -> Result<Json<ResourceList<DocumentIndexTemplate>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> document_index_templates::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = document_index_templates::table
            .filter(document_index_templates::document_index_id.eq(document_index_id))
            .into_boxed();

        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(document_index_templates::template.ilike(pattern));
        }

        if let Some(ref parent_id_str) = params.parent_id {
            if parent_id_str == "null" {
                query = query.filter(document_index_templates::parent_id.is_null());
            } else if let Ok(parent_id) = parent_id_str.parse::<i64>() {
                query = query.filter(document_index_templates::parent_id.eq(parent_id));
            }
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| {
            ApiError::new(
                diesel_to_http(e),
                "Failed to count document_index_templates",
            )
        })?;

    let mut query: document_index_templates::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentIndexTemplateSortField::Template), Some(true)) => query.order((
            document_index_templates::template.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::Template), _) => query.order((
            document_index_templates::template.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::IsLeaf), Some(true)) => query.order((
            document_index_templates::is_leaf.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::IsLeaf), _) => query.order((
            document_index_templates::is_leaf.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::Enabled), Some(true)) => query.order((
            document_index_templates::enabled.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::Enabled), _) => query.order((
            document_index_templates::enabled.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::ParentId), Some(true)) => query.order((
            document_index_templates::parent_id.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::ParentId), _) => query.order((
            document_index_templates::parent_id.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::CreatedAt), Some(true)) => query.order((
            document_index_templates::created_at.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::CreatedAt), _) => query.order((
            document_index_templates::created_at.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::UpdatedAt), Some(true)) => query.order((
            document_index_templates::updated_at.desc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::UpdatedAt), _) => query.order((
            document_index_templates::updated_at.asc(),
            document_index_templates::id.asc(),
        )),
        (Some(DocumentIndexTemplateSortField::Id), Some(true)) => {
            query.order(document_index_templates::id.desc())
        }
        _ => query.order(document_index_templates::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(DocumentIndexTemplate::as_select())
        .load::<DocumentIndexTemplate>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list document_index_templates"))?;

    Ok(Json(ResourceList {
        total,
        page,
        per_page,
        items,
    }))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{document_index_id}/templates", get(list).post(create))
        .route(
            "/{document_index_id}/templates/{id}",
            get(get_by_id).patch(update).delete(delete),
        )
}
