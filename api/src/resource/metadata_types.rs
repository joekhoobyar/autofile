use crate::Db;
use crate::schema::{metadata_types};
use crate::util::{diesel_to_http, err, ApiResult, ResourceList};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MetadataType {
    id: i64,

    slug: String,
    name: String,
    data_type: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    description: Option<String>
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewMetadataType {
    slug: String,
    name: String,
    data_type: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct MetadataTypeChangeset {
    name: Option<String>,
    data_type: Option<String>,
    description: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(FromForm)]
pub struct ListMetadataTypesQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
}

#[get("/<id>")]
pub async fn get(mut db: Connection<Db>, id: i64) -> ApiResult<Json<MetadataType>> {
    let row = metadata_types::table
        .find(id)
        .select(MetadataType::as_select())
        .first::<MetadataType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch metadata_type"))?;

    Ok(Json(row))
}

#[get("/by-slug/<slug>")]
pub async fn get_by_slug(mut db: Connection<Db>, slug: &str) -> ApiResult<Json<MetadataType>> {
    let row = metadata_types::table
        .filter(metadata_types::slug.eq(slug))
        .select(MetadataType::as_select())
        .first::<MetadataType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch metadata_type"))?;

    Ok(Json(row))
}

#[post("/", format = "json", data = "<input>")]
async fn create(mut db: Connection<Db>, input: Json<NewMetadataType>) -> ApiResult<Json<MetadataType>> {
    let inserted: MetadataType = diesel::insert_into(metadata_types::table)
        .values(&*input)
        .returning(MetadataType::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to create metadata_type"))?;

    Ok(Json(inserted))
}

#[patch("/<id>", format = "json", data = "<input>")]
async fn update(mut db: Connection<Db>, id: i64, input: Json<MetadataTypeChangeset>) -> ApiResult<Json<MetadataType>> {
    let mut changes = input.into_inner();
    changes.updated_at = Some(Utc::now());

    // Update + return the updated row in one round-trip.
    let updated: MetadataType =
        diesel::update(metadata_types::table.filter(metadata_types::id.eq(id)))
            .set(&changes)
            .returning(MetadataType::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| err(diesel_to_http(e), "failed to update metadata_type"))?;

    Ok(Json(updated))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, params: ListMetadataTypesQuery) -> ApiResult<Json<ResourceList<MetadataType>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter =  || -> metadata_types::BoxedQuery<'_, diesel::pg::Pg> {
        // Start with a boxed query so we can conditionally add filters.
        let query= metadata_types::table.into_boxed();

        // Optional search: case-insensitive substring on slug/name/description
        if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query.filter(
                metadata_types::slug.ilike(pattern.clone())
                    .or(metadata_types::name.ilike(pattern.clone()))
                    .or(metadata_types::data_type.ilike(pattern.clone()))
                    .or(metadata_types::description.ilike(pattern)),
            )
        } else { 
            query
        }
    };


    let total = base_filter()
        .count()
        .get_result::<i64>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to count metadata_types"))?;

    let items= base_filter()
        .order(metadata_types::id.desc())
        .limit(per_page)
        .offset(offset)
        .select(MetadataType::as_select())
        .load::<MetadataType>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list metadata_types"))?;

    Ok(Json(ResourceList { total, page, per_page, items }))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile metadata_types", |rocket| async {
        rocket.mount("/metadata-types", routes![list, get, get_by_slug, create, update])
    })
}
