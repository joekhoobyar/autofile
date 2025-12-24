use crate::Db;
use crate::schema::{document_types_metadata_types, metadata_types};
use crate::util::{diesel_to_http, err, ApiResult};
use crate::resource::document_types::{DocumentType};
use crate::resource::metadata_types::{MetadataType};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

#[derive(Debug, Serialize, Identifiable, Associations, Queryable, Selectable)]
#[diesel(belongs_to(DocumentType))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(primary_key(document_type_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}

#[derive(FromForm)]
pub struct ListDocumentTypesMetadataTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    pub document_type_id: Option<i64>,
    pub metadata_type_id: Option<i64>,
}

#[post("/", format = "json", data = "<input>")]
async fn create(mut db: Connection<Db>, input: Json<NewDocumentTypeMetadataType>) -> ApiResult<Json<DocumentTypeMetadataType>> {
    let inserted: DocumentTypeMetadataType = diesel::insert_into(document_types_metadata_types::table)
        .values(&*input)
        .returning(DocumentTypeMetadataType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to create document_type_metadata_type"))?;

    Ok(Json(inserted))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, params: ListDocumentTypesMetadataTypesQuery) -> ApiResult<Json<Vec<DocumentTypeMetadataType>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = document_types_metadata_types::table.inner_join(metadata_types::table).into_boxed();

    // Optional search: case-insensitive substring on slug/name/description
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query
            .filter(
                metadata_types::slug.ilike(pattern.clone())
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
        .order((document_types_metadata_types::document_type_id.desc(), document_types_metadata_types::metadata_type_id.desc()))
        .limit(per_page)
        .offset(offset)
        .select(DocumentTypeMetadataType::as_select())
        .load::<DocumentTypeMetadataType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list document_types_metadata_types"))?;

    Ok(Json(rows))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile document_types_metadata_types", |rocket| async {
        rocket.mount("/document-types-metadata-types", routes![list, create])
    })
}
