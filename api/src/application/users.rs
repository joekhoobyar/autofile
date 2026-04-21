use crate::domain::users::User;
use crate::schema::users;
use crate::shared::util::{ApiError, ResourceList, diesel_to_http};

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

const SYSTEM_USER_ID: i64 = 1;

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserSortField {
    Id,
    Username,
    Email,
    DisplayName,
    CreatedAt,
    UpdatedAt,
    PasswordChangedAt,
}

#[derive(Debug, serde::Deserialize)]
pub struct ListUsersInput {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub q: Option<String>,
    pub sf: Option<UserSortField>,
    pub sd: Option<bool>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserChangeset {
    email: Option<String>,
    display_name: Option<String>,
}

pub async fn get_user_by_id(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<User, ApiError> {
    users::table
        .find(id)
        .select(User::as_select())
        .first::<User>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch user"))
}

pub async fn get_user_by_username(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    username: String,
) -> Result<User, ApiError> {
    users::table
        .filter(users::username.eq(username))
        .select(User::as_select())
        .first::<User>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to fetch user"))
}

pub async fn update_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
    input: UpdateUserInput,
) -> Result<User, ApiError> {
    if id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot update system user"));
    }

    let changes = UserChangeset {
        email: input.email,
        display_name: input.display_name,
    };

    diesel::update(users::table.filter(users::id.eq(id)))
        .set((&changes, users::updated_at.eq(diesel::dsl::now)))
        .returning(User::as_returning())
        .get_result(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to update user"))
}

pub async fn delete_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<(), ApiError> {
    if id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot delete system user"));
    }

    let affected = diesel::delete(users::table.filter(users::id.eq(id)))
        .execute(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to delete user"))?;

    if affected == 0 {
        return Err(ApiError::not_found("User not found"));
    }

    Ok(())
}

pub async fn list_users(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    input: ListUsersInput,
) -> Result<ResourceList<User>, ApiError> {
    let page = input.page.unwrap_or(1).max(1);
    let per_page = input.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    let base_filter = || -> users::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = users::table.into_boxed();

        if let Some(q) = input.q.as_deref().filter(|s| !s.is_empty()) {
            let pattern = format!("%{}%", q);
            query = query.filter(
                users::username
                    .ilike(pattern.clone())
                    .or(users::display_name.ilike(pattern.clone()))
                    .or(users::email.ilike(pattern)),
            );
        }

        query
    };

    let total = base_filter()
        .count()
        .get_result::<i64>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to count users"))?;

    let mut query: users::BoxedQuery<'_, diesel::pg::Pg> = base_filter();
    query = match (input.sf, input.sd) {
        (Some(UserSortField::Username), Some(true)) => {
            query.order((users::username.desc(), users::id.asc()))
        }
        (Some(UserSortField::Username), _) => query.order((users::username.asc(), users::id.asc())),
        (Some(UserSortField::Email), Some(true)) => {
            query.order((users::email.desc(), users::id.asc()))
        }
        (Some(UserSortField::Email), _) => query.order((users::email.asc(), users::id.asc())),
        (Some(UserSortField::DisplayName), Some(true)) => {
            query.order((users::display_name.desc(), users::id.asc()))
        }
        (Some(UserSortField::DisplayName), _) => {
            query.order((users::display_name.asc(), users::id.asc()))
        }
        (Some(UserSortField::CreatedAt), Some(true)) => {
            query.order((users::created_at.desc(), users::id.asc()))
        }
        (Some(UserSortField::CreatedAt), _) => {
            query.order((users::created_at.asc(), users::id.asc()))
        }
        (Some(UserSortField::UpdatedAt), Some(true)) => {
            query.order((users::updated_at.desc(), users::id.asc()))
        }
        (Some(UserSortField::UpdatedAt), _) => {
            query.order((users::updated_at.asc(), users::id.asc()))
        }
        (Some(UserSortField::PasswordChangedAt), Some(true)) => {
            query.order((users::password_changed_at.desc(), users::id.asc()))
        }
        (Some(UserSortField::PasswordChangedAt), _) => {
            query.order((users::password_changed_at.asc(), users::id.asc()))
        }
        (Some(UserSortField::Id), Some(true)) => query.order(users::id.desc()),
        _ => query.order(users::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(User::as_select())
        .load::<User>(db)
        .await
        .map_err(|e| ApiError::new(diesel_to_http(e), "Failed to list users"))?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
