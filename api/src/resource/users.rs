use crate::Db;
use crate::schema::{users};
use crate::util::{diesel_to_http, err, ApiResult};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};
use diesel::prelude::*; // macros + schema/table dsl live here

#[derive(Debug, Serialize, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    id: i64,
    username: String,
    display_name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewUser {
    username: String,
    display_name: String,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserChangeset {
    display_name: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(FromForm)]
pub struct ListUsersQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
}

#[get("/<id>")]
pub async fn get(mut db: Connection<Db>, id: i64) -> ApiResult<Json<User>> {
    let row = users::table
        .find(id)
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch user"))?;

    Ok(Json(row))
}

#[get("/by-username/<username>")]
pub async fn get_by_username(mut db: Connection<Db>, username: &str) -> ApiResult<Json<User>> {
    let row = users::table
        .filter(users::username.eq(username))
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch user"))?;

    Ok(Json(row))
}

#[post("/", format = "json", data = "<input>")]
async fn create(mut db: Connection<Db>, input: Json<NewUser>) -> ApiResult<Json<User>> {
    let inserted: User = diesel::insert_into(users::table)
        .values(&*input)
        .returning(User::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to create user"))?;

    Ok(Json(inserted))
}

#[patch("/<id>", format = "json", data = "<input>")]
async fn update(mut db: Connection<Db>, id: i64, input: Json<UserChangeset>) -> ApiResult<Json<User>> {
    let mut changes = input.into_inner();
    changes.updated_at = Some(Utc::now());

    // Update + return the updated row in one round-trip.
    let updated: User =
        diesel::update(users::table.filter(users::id.eq(id)))
            .set(&changes)
            .returning(User::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| err(diesel_to_http(e), "failed to update user"))?;

    Ok(Json(updated))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, params: ListUsersQuery) -> ApiResult<Json<Vec<User>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = users::table.into_boxed();

    // Optional search: case-insensitive substring on username/display_name
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query.filter(
            users::username.ilike(pattern.clone()).or(users::display_name.ilike(pattern)),
        );
    }

    let rows = query
        .order(users::id.desc())
        .limit(per_page)
        .offset(offset)
        .select(User::as_select())
        .load::<User>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list users"))?;

    Ok(Json(rows))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile Users", |rocket| async {
        rocket.mount("/users", routes![list, get, get_by_username, create, update])
    })
}
