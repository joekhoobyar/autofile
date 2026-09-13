use axum::http::StatusCode;
use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::domain::document_indexes::DocumentIndexTemplate;
use crate::schema::document_index_templates;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;
use crate::shared::serde::de_present_option;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDocumentIndexTemplate {
    pub template: String,
    pub is_leaf: bool,
    pub enabled: bool,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = document_index_templates)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentIndexTemplateChangeset {
    pub template: Option<String>,
    pub is_leaf: Option<bool>,
    pub enabled: Option<bool>,

    /// Set a new parent, or null to detach. Omitted when unchanged.
    #[serde(default, deserialize_with = "de_present_option")]
    #[schema(value_type = Option<i64>)]
    pub parent_id: Option<Option<i64>>,
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

pub async fn get_document_index_template(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    id: i64,
) -> Result<DocumentIndexTemplate, ApiError> {
    document_index_templates::table
        .filter(document_index_templates::document_index_id.eq(document_index_id))
        .filter(document_index_templates::id.eq(id))
        .select(DocumentIndexTemplate::as_select())
        .first::<DocumentIndexTemplate>(db)
        .await
        .api_context("Failed to fetch document_index_template")
}

pub async fn create_document_index_template(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    document_index_id: i64,
    input: NewDocumentIndexTemplate,
) -> Result<DocumentIndexTemplate, ApiError> {
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

    diesel::insert_into(document_index_templates::table)
        .values((
            &input,
            document_index_templates::document_index_id.eq(document_index_id),
            document_index_templates::created_by.eq(user_id),
            document_index_templates::updated_by.eq(user_id),
        ))
        .returning(DocumentIndexTemplate::as_returning())
        .get_result(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to create document_index_template", e))
}

pub async fn update_document_index_template(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    document_index_id: i64,
    id: i64,
    input: DocumentIndexTemplateChangeset,
) -> Result<DocumentIndexTemplate, ApiError> {
    let common = (
        input
            .template
            .map(|v| document_index_templates::template.eq(v)),
        input
            .is_leaf
            .map(|v| document_index_templates::is_leaf.eq(v)),
        input
            .enabled
            .map(|v| document_index_templates::enabled.eq(v)),
        document_index_templates::updated_at.eq(diesel::dsl::now),
        document_index_templates::updated_by.eq(user_id),
    );

    let base = diesel::update(
        document_index_templates::table
            .filter(document_index_templates::document_index_id.eq(document_index_id))
            .filter(document_index_templates::id.eq(id)),
    );

    let base = match input.parent_id {
        None => {
            base.set(common)
                .returning(DocumentIndexTemplate::as_returning())
                .get_result(db)
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
                .get_result(db)
                .await
        }
        Some(None) => {
            base.set((
                common,
                document_index_templates::parent_id.eq::<Option<i64>>(None),
            ))
            .returning(DocumentIndexTemplate::as_returning())
            .get_result(db)
            .await
        }
    };

    base.map_err(|e| ApiError::from_diesel("Failed to update document_index_template", e))
}

pub async fn delete_document_index_template(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(
        document_index_templates::table
            .filter(document_index_templates::document_index_id.eq(document_index_id))
            .filter(document_index_templates::id.eq(id)),
    )
    .execute(db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to delete document_index_template", e))?;

    if affected == 0 {
        return Err(ApiError::not_found("Document index template not found"));
    }

    Ok(())
}

pub async fn list_document_index_templates(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_index_id: i64,
    params: ListDocumentIndexTemplatesQuery,
) -> Result<ResourceList<DocumentIndexTemplate>, ApiError> {
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
        .get_result::<i64>(db)
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
        .load::<DocumentIndexTemplate>(db)
        .await
        .api_context("Failed to list document_index_templates")?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
