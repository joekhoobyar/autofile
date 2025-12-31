use crate::Db;
use crate::schema::{document_types};
use crate::util::{diesel_to_http, err, ApiResult, ResourceList};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentType {
    id: i64,

    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    description: Option<String>
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentType {
    slug: String,
    name: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = document_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentTypeChangeset {
    name: Option<String>,
    description: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, FromFormField)]
pub enum DocumentTypeSortField {
    #[field(value="id")]
    Id,
    #[field(value="slug")]
    Slug,
    #[field(value="name")]
    Name,
    #[field(value="description")]
    Description,
    #[field(value="created_at")]
    CreatedAt,
    #[field(value="updated_at")]
    UpdatedAt,
}

#[derive(FromForm)]
pub struct ListDocumentTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    // optional sort field
    pub sf: Option<DocumentTypeSortField>,
    // optional sort descending
    pub sd: Option<bool>,
}

#[get("/<id>")]
pub async fn get(mut db: Connection<Db>, id: i64) -> ApiResult<Json<DocumentType>> {
    let row = document_types::table
        .find(id)
        .select(DocumentType::as_select())
        .first::<DocumentType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch document_type"))?;

    Ok(Json(row))
}

#[get("/by-slug/<slug>")]
pub async fn get_by_slug(mut db: Connection<Db>, slug: &str) -> ApiResult<Json<DocumentType>> {
    let row = document_types::table
        .filter(document_types::slug.eq(slug))
        .select(DocumentType::as_select())
        .first::<DocumentType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch document_type"))?;

    Ok(Json(row))
}

#[post("/", format = "json", data = "<input>")]
async fn create(mut db: Connection<Db>, input: Json<NewDocumentType>) -> ApiResult<Json<DocumentType>> {
    let inserted: DocumentType = diesel::insert_into(document_types::table)
        .values(&*input)
        .returning(DocumentType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to create document_type"))?;

    Ok(Json(inserted))
}

#[patch("/<id>", format = "json", data = "<input>")]
async fn update(mut db: Connection<Db>, id: i64, input: Json<DocumentTypeChangeset>) -> ApiResult<Json<DocumentType>> {
    let mut changes = input.into_inner();
    changes.updated_at = Some(Utc::now());

    // Update + return the updated row in one round-trip.
    let updated: DocumentType =
        diesel::update(document_types::table.filter(document_types::id.eq(id)))
            .set(&changes)
            .returning(DocumentType::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| err(diesel_to_http(e), "failed to update document_type"))?;

    Ok(Json(updated))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, params: ListDocumentTypesQuery) -> ApiResult<Json<ResourceList<DocumentType>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter =  || -> document_types::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let query= document_types::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query.filter(
                document_types::slug.ilike(pattern.clone())
                    .or(document_types::name.ilike(pattern.clone()))
                    .or(document_types::description.ilike(pattern)),
            )
        } else { 
            query
        }
    };


    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to count document_types"))?;

    let mut query: document_types::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (params.sf, params.sd) {
        (Some(DocumentTypeSortField::Slug), Some(true)) =>
            query.order((document_types::slug.desc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::Slug), _) =>
            query.order((document_types::slug.asc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::Name), Some(true)) =>
            query.order((document_types::name.desc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::Name), _) =>
            query.order((document_types::name.asc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::Description), Some(true)) =>
            query.order((document_types::description.desc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::Description), _) =>
            query.order((document_types::description.asc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::CreatedAt), Some(true)) =>
            query.order((document_types::created_at.desc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::CreatedAt), _) =>
            query.order((document_types::created_at.asc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::UpdatedAt), Some(true)) =>
            query.order((document_types::updated_at.desc(), document_types::id.asc())), // tie-breaker
        (Some(DocumentTypeSortField::UpdatedAt), _) =>
            query.order((document_types::updated_at.asc(), document_types::id.asc())), // tie-breaker

        (Some(DocumentTypeSortField::Id), Some(true)) =>
            query.order(document_types::id.desc()),
        _ =>
            query.order(document_types::id.asc()),
    };

    let items= query
        .limit(per_page)
        .offset(offset)
        .select(DocumentType::as_select())
        .load::<DocumentType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list document_types"))?;

    Ok(Json(ResourceList { total, page, per_page, items }))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile document_types", |rocket| async {
        rocket.mount("/document-types", routes![list, get, get_by_slug, create, update])
    })
}
