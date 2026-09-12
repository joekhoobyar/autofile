use std::sync::Arc;

use crate::domain::document_types_metadata_types::DocumentTypeMetadataType;
use crate::schema::{document_types_metadata_types, metadata_types};
use crate::shared::app_state::AppState;
use crate::shared::auth::{AdminUser, AuthUser};
use crate::shared::extractors::DbConn;
use crate::shared::util::ApiError;

use serde::Deserialize;

use axum::{
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use utoipa_axum::{router::OpenApiRouter, routes};

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentTypeNewMetadataTypeInput {
    metadata_type_id: i64,
    required: bool,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentTypeMetadataTypeChangeset {
    required: Option<bool>,
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListDocumentTypesMetadataTypesQuery {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200). No total is returned.
    pub per_page: Option<i64>,
    /// Case-insensitive search of the associated metadata type's slug, name, data type, and description.
    pub q: Option<String>,
    /// Narrow to one document type.
    pub document_type_id: Option<i64>,
    /// Narrow to one metadata type.
    pub metadata_type_id: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Association", body = DocumentTypeMetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
pub async fn get_by_ids(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let row = document_types_metadata_types::table
        .find((document_type_id, metadata_type_id))
        .select(DocumentTypeMetadataType::as_select())
        .first::<DocumentTypeMetadataType>(&mut db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document_type_metadata_type", e))?;

    Ok(Json(row))
}

#[utoipa::path(
    post,
    path = "/",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    request_body = NewDocumentTypeMetadataType,
    responses(
        (status = 200, description = "Created association", body = DocumentTypeMetadataType),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 409, description = "Association already exists", body = ApiError),
        (status = 422, description = "Unknown document or metadata type", body = ApiError),
    )
)]
async fn create(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewDocumentTypeMetadataType>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let inserted: DocumentTypeMetadataType =
        diesel::insert_into(document_types_metadata_types::table)
            .values(&input)
            .returning(DocumentTypeMetadataType::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| {
                ApiError::from_diesel("Failed to create document_type_metadata_type", e)
            })?;

    Ok(Json(inserted))
}

#[utoipa::path(
    patch,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    request_body = DocumentTypeMetadataTypeChangeset,
    responses(
        (status = 200, description = "Updated association", body = DocumentTypeMetadataType),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn update(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
    Json(input): Json<DocumentTypeMetadataTypeChangeset>,
) -> Result<Json<DocumentTypeMetadataType>, ApiError> {
    let changes = input;

    // Update + return the updated row in one round-trip.
    let updated: DocumentTypeMetadataType = diesel::update(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
            .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id)),
    )
    .set((
        &changes,
        document_types_metadata_types::updated_at.eq(diesel::dsl::now),
    ))
    .returning(DocumentTypeMetadataType::as_returning())
    .get_result(&mut db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to update document_type_metadata_type", e))?;

    Ok(Json(updated))
}

#[utoipa::path(
    post,
    path = "/{document_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(("document_type_id" = i64, Path, description = "Document Type ID")),
    request_body = Vec<DocumentTypeNewMetadataTypeInput>,
    responses(
        (status = 200, description = "Replacement association set; an empty array removes all associations", body = Vec<DocumentTypeMetadataType>),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 422, description = "Unknown metadata type", body = ApiError),
    )
)]
async fn document_type_save(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path(document_type_id): Path<i64>,
    Json(input): Json<Vec<DocumentTypeNewMetadataTypeInput>>,
) -> Result<Json<Vec<DocumentTypeMetadataType>>, ApiError> {
    let rows = db
        .transaction::<_, diesel::result::Error, _>(async move |conn| {
            diesel::delete(
                document_types_metadata_types::table
                    .filter(document_types_metadata_types::document_type_id.eq(document_type_id)),
            )
            .execute(conn)
            .await?;

            if input.is_empty() {
                return Ok(Vec::new());
            }

            // Prepare the rows to insert. It is worth allocating memory so that we can
            // bulk insert with Diesel, rather than doing individual queries in a loop.
            let rows: Vec<NewDocumentTypeMetadataType> = input
                .into_iter()
                .map(|m| NewDocumentTypeMetadataType {
                    document_type_id,
                    metadata_type_id: m.metadata_type_id,
                    required: m.required,
                })
                .collect();

            let rows = diesel::insert_into(document_types_metadata_types::table)
                .values(&rows)
                .returning(DocumentTypeMetadataType::as_returning())
                .get_results::<DocumentTypeMetadataType>(conn)
                .await?;

            Ok(rows)
        })
        .await
        .map_err(|e| ApiError::from_diesel("Failed to save document_type_metadata_type", e))?;

    Ok(Json(rows))
}

#[utoipa::path(
    delete,
    path = "/{document_type_id}/{metadata_type_id}",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(
        ("document_type_id" = i64, Path, description = "Document Type ID"),
        ("metadata_type_id" = i64, Path, description = "Metadata Type ID"),
    ),
    responses(
        (status = 200, description = "Association removed"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Admin role required", body = ApiError),
        (status = 404, description = "Association not found", body = ApiError),
    )
)]
async fn delete_junction(
    _user: AdminUser,
    DbConn(mut db): DbConn,
    Path((document_type_id, metadata_type_id)): Path<(i64, i64)>,
) -> Result<Json<()>, ApiError> {
    let affected = diesel::delete(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
            .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id)),
    )
    .execute(&mut db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to delete document_type_metadata_type", e))?;

    if affected == 0 {
        return Err(ApiError::not_found("document_type_metadata_type not found"));
    }

    Ok(Json(()))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "document-types-metadata-types",
    security(("bearer" = [])),
    params(ListDocumentTypesMetadataTypesQuery),
    responses(
        (status = 200, description = "Association list as a bare array (no total)", body = Vec<DocumentTypeMetadataType>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentTypesMetadataTypesQuery>,
) -> Result<Json<Vec<DocumentTypeMetadataType>>, ApiError> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = document_types_metadata_types::table
        .inner_join(metadata_types::table)
        .into_boxed();

    // Optional search: case-insensitive substring on slug/name/data_type/description
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query.filter(
            metadata_types::slug
                .ilike(pattern.clone())
                .or(metadata_types::name.ilike(pattern.clone()))
                .or(metadata_types::data_type.ilike(pattern.clone()))
                .or(metadata_types::description.ilike(pattern)),
        );
    }

    // Filter by document type or metadata type
    if let Some(id) = params.document_type_id {
        query = query.filter(document_types_metadata_types::document_type_id.eq(id));
    }
    if let Some(id) = params.metadata_type_id {
        query = query.filter(document_types_metadata_types::metadata_type_id.eq(id));
    }

    let rows = query
        .order((
            document_types_metadata_types::document_type_id.desc(),
            document_types_metadata_types::metadata_type_id.desc(),
        ))
        .limit(per_page)
        .offset(offset)
        .select(DocumentTypeMetadataType::as_select())
        .load::<DocumentTypeMetadataType>(&mut db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_types_metadata_types", e))?;

    Ok(Json(rows))
}

pub fn routes() -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        .routes(routes!(document_type_save))
        .routes(routes!(get_by_ids))
        .routes(routes!(update))
        .routes(routes!(delete_junction))
}
