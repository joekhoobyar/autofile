use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use serde::Deserialize;

use crate::domain::document_types_metadata_types::DocumentTypeMetadataType;
use crate::schema::{document_types_metadata_types, metadata_types};
use crate::shared::errors::ApiError;

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDocumentTypeMetadataType {
    pub document_type_id: i64,
    pub metadata_type_id: i64,
    pub required: bool,
}

#[derive(Debug, Deserialize, Insertable, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeNewMetadataTypeInput {
    pub metadata_type_id: i64,
    pub required: bool,
}

#[derive(Debug, Deserialize, AsChangeset, utoipa::ToSchema)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeMetadataTypeChangeset {
    pub required: Option<bool>,
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

pub async fn get_document_type_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_type_id: i64,
    metadata_type_id: i64,
) -> Result<DocumentTypeMetadataType, ApiError> {
    document_types_metadata_types::table
        .find((document_type_id, metadata_type_id))
        .select(DocumentTypeMetadataType::as_select())
        .first::<DocumentTypeMetadataType>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document_type_metadata_type", e))
}

pub async fn create_document_type_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    input: NewDocumentTypeMetadataType,
) -> Result<DocumentTypeMetadataType, ApiError> {
    diesel::insert_into(document_types_metadata_types::table)
        .values(&input)
        .returning(DocumentTypeMetadataType::as_returning())
        .get_result(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to create document_type_metadata_type", e))
}

pub async fn update_document_type_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_type_id: i64,
    metadata_type_id: i64,
    input: DocumentTypeMetadataTypeChangeset,
) -> Result<DocumentTypeMetadataType, ApiError> {
    // Update + return the updated row in one round-trip.
    diesel::update(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
            .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id)),
    )
    .set((
        &input,
        document_types_metadata_types::updated_at.eq(diesel::dsl::now),
    ))
    .returning(DocumentTypeMetadataType::as_returning())
    .get_result(db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to update document_type_metadata_type", e))
}

pub async fn save_document_type_metadata_types(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_type_id: i64,
    input: Vec<DocumentTypeNewMetadataTypeInput>,
) -> Result<Vec<DocumentTypeMetadataType>, ApiError> {
    db.transaction::<_, diesel::result::Error, _>(async move |conn| {
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
    .map_err(|e| ApiError::from_diesel("Failed to save document_type_metadata_type", e))
}

pub async fn delete_document_type_metadata_type(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_type_id: i64,
    metadata_type_id: i64,
) -> Result<(), ApiError> {
    let affected = diesel::delete(
        document_types_metadata_types::table
            .filter(document_types_metadata_types::document_type_id.eq(document_type_id))
            .filter(document_types_metadata_types::metadata_type_id.eq(metadata_type_id)),
    )
    .execute(db)
    .await
    .map_err(|e| ApiError::from_diesel("Failed to delete document_type_metadata_type", e))?;

    if affected == 0 {
        return Err(ApiError::not_found("document_type_metadata_type not found"));
    }

    Ok(())
}

pub async fn list_document_types_metadata_types(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    params: ListDocumentTypesMetadataTypesQuery,
) -> Result<Vec<DocumentTypeMetadataType>, ApiError> {
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

    query
        .order((
            document_types_metadata_types::document_type_id.desc(),
            document_types_metadata_types::metadata_type_id.desc(),
        ))
        .limit(per_page)
        .offset(offset)
        .select(DocumentTypeMetadataType::as_select())
        .load::<DocumentTypeMetadataType>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_types_metadata_types", e))
}
