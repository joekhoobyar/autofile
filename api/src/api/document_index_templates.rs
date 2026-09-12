use std::sync::Arc;

use crate::domain::document_indexes::DocumentIndexTemplate;
use crate::schema::document_index_templates;
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;
use crate::shared::serde::de_present_option;

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
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentIndexTemplate {
    template: String,
    is_leaf: bool,
    enabled: bool,
    parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentIndexTemplateChangeset {
    template: Option<String>,
    is_leaf: Option<bool>,
    enabled: Option<bool>,

    /// Set a new parent, or null to detach. Omitted when unchanged.
    #[serde(default, deserialize_with = "de_present_option")]
    #[schema(value_type = Option<i64>)]
    parent_id: Option<Option<i64>>,
}

#[derive(Debug, Clone, Copy, Deserialize, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentIndexTemplatesQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive template substring search.
    pub q: Option<String>,
    /// Filter by parent: "null" for top-level templates, or a numeric ID.
    pub parent_id: Option<String>,
    /// Sort field.
    pub sf: Option<DocumentIndexTemplateSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Document Index Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
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
        .api_context("Failed to fetch document_index_template")?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/{document_index_id}/templates",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(("document_index_id" = i64, Path, description = "Document Index ID")),
    request_body = NewDocumentIndexTemplate,
    responses(
        (status = 200, description = "Created Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document Index not found", body = ApiError),
        (status = 422, description = "Invalid document index or parent template", body = ApiError),
    )
)]
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

    if let Some(parent_id) = input.parent_id
        && parent_id <= 0
    {
        return Err(ApiError::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Invalid parent template",
        ));
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
        .map_err(|e| ApiError::from_diesel("Failed to create document_index_template", e))?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    request_body = DocumentIndexTemplateChangeset,
    responses(
        (status = 200, description = "Updated Template", body = DocumentIndexTemplate),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
        (status = 422, description = "Invalid parent template", body = ApiError),
    )
)]
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

    let updated: DocumentIndexTemplate =
        base.map_err(|e| ApiError::from_diesel("Failed to update document_index_template", e))?;

    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/{document_index_id}/templates/{id}",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ("id" = i64, Path, description = "Template ID"),
    ),
    responses(
        (status = 200, description = "Template deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
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
    .map_err(|e| ApiError::from_diesel("Failed to delete document_index_template", e))?;

    if affected == 0 {
        return Err(ApiError::not_found("Document index template not found"));
    }

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/{document_index_id}/templates",
    tag = "document-indexes",
    security(("bearer" = [])),
    params(
        ("document_index_id" = i64, Path, description = "Document Index ID"),
        ListDocumentIndexTemplatesQuery,
    ),
    responses(
        (status = 200, description = "Paginated Template list", body = ResourceList<DocumentIndexTemplate>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
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
        .map_err(|e| ApiError::from_diesel("Failed to count document_index_templates", e))?;

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
        .api_context("Failed to list document_index_templates")?;

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
}
