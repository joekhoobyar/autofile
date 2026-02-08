use std::sync::Arc;

use crate::AppState;
use crate::domain::users::User;
use crate::schema::users;
use crate::shared::auth::AuthUser;
use crate::shared::extractors::DbConn;
use crate::shared::util::{diesel_to_http, ApiError};

use serde::Deserialize;

use axum::{
    Router,
    routing::get,
    Json,
    extract::{Path, Query},
};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use chrono::{DateTime, Utc};

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserChangeset {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
}

pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<User>, ApiError> {
    let row = users::table
        .find(id)
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch user"))?;

    Ok(Json(row))
}

pub async fn get_by_username(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(username): Path<String>,
) -> Result<Json<User>, ApiError> {
    let row = users::table
        .filter(users::username.eq(username))
        .select(User::as_select())
        .first::<User>(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch user"))?;

    Ok(Json(row))
}

async fn create(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Json(input): Json<NewUser>,
) -> Result<Json<User>, ApiError> {
    let inserted: User = diesel::insert_into(users::table)
        .values(&input)
        .returning(User::as_returning())
        .get_result(&mut db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to create user"))?;

    Ok(Json(inserted))
}

async fn update(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<UserChangeset>,
) -> Result<Json<User>, ApiError> {
    let mut changes = input;
    changes.updated_at = Some(Utc::now());

    // Update + return the updated row in one round-trip.
    let updated: User =
        diesel::update(users::table.filter(users::id.eq(id)))
            .set(&changes)
            .returning(User::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update user"))?;

    Ok(Json(updated))
}

pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<Vec<User>>, ApiError> {
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
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list users"))?;

    Ok(Json(rows))
}

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", get(get_by_id).patch(update))
        .route("/by-username/{username}", get(get_by_username))
}
