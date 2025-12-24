use crate::Db;
use crate::schema::{cabinets};
use crate::util::{ApiResult, FormFieldPresence, diesel_to_http, err, de_present_option};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Cabinet {
    id: i64,

    slug: String,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    description: Option<String>,
    parent_id: Option<i64>
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewCabinet {
    slug: String,
    name: String,
    description: Option<String>,
    parent_id: Option<i64>,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = cabinets)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct CabinetChangeset {
    name: Option<String>,
    description: Option<String>,

    #[serde(default, deserialize_with = "de_present_option")]
    parent_id: Option<Option<i64>>,

    updated_at: Option<DateTime<Utc>>,
}

#[derive(FromForm)]
pub struct ListCabinetsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    pub parent_id: Option<FormFieldPresence<i64>>,
}

#[get("/<id>")]
pub async fn get(mut db: Connection<Db>, id: i64) -> ApiResult<Json<Cabinet>> {
    let row = cabinets::table
        .find(id)
        .select(Cabinet::as_select())
        .first::<Cabinet>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch cabinet"))?;

    Ok(Json(row))
}

#[get("/by-slug/<slug>")]
pub async fn get_by_slug(mut db: Connection<Db>, slug: &str) -> ApiResult<Json<Cabinet>> {
    let row = cabinets::table
        .filter(cabinets::slug.eq(slug))
        .select(Cabinet::as_select())
        .first::<Cabinet>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch cabinet"))?;

    Ok(Json(row))
}

#[post("/", format = "json", data = "<input>")]
async fn create(mut db: Connection<Db>, input: Json<NewCabinet>) -> ApiResult<Json<Cabinet>> {
    if let Some(parent_id) = input.parent_id && parent_id <= 0 {
        return Err(err(
            rocket::http::Status::UnprocessableEntity,
            "invalid parent cabinet",
        ));
    }

    let inserted: Cabinet = diesel::insert_into(cabinets::table)
        .values(&*input)
        .returning(Cabinet::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to create cabinet"))?;

    Ok(Json(inserted))
}

#[patch("/<id>", format = "json", data = "<input>")]
async fn update(mut db: Connection<Db>, id: i64, input: Json<CabinetChangeset>) -> ApiResult<Json<Cabinet>> {
    let patch = input.into_inner();

    // Common assignments (no parent_id here)
    let common = (
        patch.name.map(|v| cabinets::name.eq(v)),
        patch.description.map(|v| cabinets::description.eq(v)),
        cabinets::updated_at.eq(Utc::now()),
    );

    let base = diesel::update(cabinets::table.filter(cabinets::id.eq(id)));

    let base = match patch.parent_id {
        None => {
            base.set(common)
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
        Some(Some(parent_id)) => {
            if parent_id <= 0 || parent_id == id {
                return Err(err(
                    rocket::http::Status::UnprocessableEntity,
                    "invalid parent cabinet",
                ));
            }
            base.set((common, cabinets::parent_id.eq(parent_id)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
        Some(None) => {
            base.set((common, cabinets::parent_id.eq::<Option<i64>>(None)))
                .returning(Cabinet::as_returning())
                .get_result(&mut db)
                .await
        },
    };

    // Update + return the updated row in one round-trip.
    let updated: Cabinet = base
        .map_err(|e| err(diesel_to_http(e), "failed to update cabinet"))?;

    Ok(Json(updated))
}

#[delete("/<id>", format = "json")]
async fn delete(mut db: Connection<Db>, id: i64) -> ApiResult<Json<()>> {

    // Update + return the updated row in one round-trip.
    let affected = diesel::delete(cabinets::table.filter(cabinets::id.eq(id)))
        .execute(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to delete cabinet"))?;

    if affected == 0 {
        return Err(err(rocket::http::Status::NotFound, "cabinet not found"));
    }

    Ok(Json(()))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, params: ListCabinetsQuery) -> ApiResult<Json<Vec<Cabinet>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = cabinets::table.into_boxed();

    // Optional search: case-insensitive substring on slug/name/description
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query.filter(
            cabinets::slug.ilike(pattern.clone())
                .or(cabinets::name.ilike(pattern.clone()))
                .or(cabinets::description.ilike(pattern)),
        );
    }

    // Optional filter by parent
    match params.parent_id {
        Some(FormFieldPresence::Null) => { 
            query = query.filter(cabinets::parent_id.is_null());
        }
        Some(FormFieldPresence::Value(v)) => {
            query = query.filter(cabinets::parent_id.eq(v));
        }
        None => ()
    }

    let rows = query
        .order(cabinets::id.desc())
        .limit(per_page)
        .offset(offset)
        .select(Cabinet::as_select())
        .load::<Cabinet>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list cabinets"))?;

    Ok(Json(rows))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile cabinets", |rocket| async {
        rocket.mount("/cabinets", routes![list, get, get_by_slug, create, update, delete])
    })
}
