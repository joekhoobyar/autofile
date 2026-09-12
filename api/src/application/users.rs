use crate::domain::users::{User, UserRole};
use crate::schema::users;
use crate::shared::auth::hash_password;
use crate::shared::errors::{ApiError, ApiErrorContext};
use crate::shared::responses::ResourceList;

use bb8::PooledConnection;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

const SYSTEM_USER_ID: i64 = 1;

#[derive(Debug, Clone, Copy, serde::Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserSortField {
    Id,
    Username,
    Email,
    DisplayName,
    CreatedAt,
    UpdatedAt,
    PasswordChangedAt,
    Enabled,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListUsersInput {
    /// 1-based page number.
    pub page: Option<i64>,
    /// Items per page (1 through 200).
    pub per_page: Option<i64>,
    /// Case-insensitive search of username, display name, and email.
    pub q: Option<String>,
    /// Sort field.
    pub sf: Option<UserSortField>,
    /// Set to true for descending order.
    pub sd: Option<bool>,
    /// Include soft-deleted users.
    pub include_deleted: Option<bool>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateUserInput {
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub role: Option<UserRole>,
    pub force_password_change: Option<bool>,
    pub enabled: Option<bool>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateProfileInput {
    pub email: Option<String>,
    pub display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangePasswordInput {
    pub new_password: String,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct UserChangeset {
    email: Option<String>,
    display_name: Option<String>,
    role: Option<UserRole>,
    force_password_change: Option<bool>,
    enabled: Option<bool>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct ProfileChangeset {
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
        .api_context("Failed to fetch user")
}

pub async fn get_user_by_username(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    username: String,
) -> Result<User, ApiError> {
    users::table
        .filter(users::username.eq(username))
        .filter(users::deleted_at.is_null())
        .select(User::as_select())
        .first::<User>(db)
        .await
        .api_context("Failed to fetch user")
}

pub async fn get_profile(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
) -> Result<User, ApiError> {
    users::table
        .filter(users::id.eq(user_id))
        .filter(users::deleted_at.is_null())
        .select(User::as_select())
        .first::<User>(db)
        .await
        .api_context("Failed to fetch user")
}

pub async fn update_profile(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: UpdateProfileInput,
) -> Result<User, ApiError> {
    if user_id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot update system user"));
    }

    let changes = ProfileChangeset {
        email: input.email,
        display_name: input.display_name,
    };

    diesel::update(
        users::table
            .filter(users::id.eq(user_id))
            .filter(users::deleted_at.is_null()),
    )
    .set((&changes, users::updated_at.eq(diesel::dsl::now)))
    .returning(User::as_returning())
    .get_result(db)
    .await
    .api_context("Failed to update profile")
}

pub async fn change_password(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    input: ChangePasswordInput,
) -> Result<User, ApiError> {
    if user_id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot update system user"));
    }

    let pw_hash = hash_password(&input.new_password).map_err(ApiError::bad_request)?;

    diesel::update(
        users::table
            .filter(users::id.eq(user_id))
            .filter(users::deleted_at.is_null()),
    )
    .set((
        users::password_hash.eq(pw_hash),
        users::password_changed_at.eq(diesel::dsl::now),
        users::force_password_change.eq(false),
        users::updated_at.eq(diesel::dsl::now),
    ))
    .returning(User::as_returning())
    .get_result(db)
    .await
    .api_context("Failed to change password")
}

pub async fn update_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    acting_user_id: i64,
    id: i64,
    input: UpdateUserInput,
) -> Result<User, ApiError> {
    if id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot update system user"));
    }

    if id == acting_user_id && input.role == Some(UserRole::User) {
        return Err(ApiError::bad_request("Cannot remove your own admin role"));
    }

    if id == acting_user_id && input.enabled == Some(false) {
        return Err(ApiError::bad_request("Cannot disable your own user"));
    }

    let changes = UserChangeset {
        email: input.email,
        display_name: input.display_name,
        role: input.role,
        force_password_change: input.force_password_change,
        enabled: input.enabled,
    };

    diesel::update(
        users::table
            .filter(users::id.eq(id))
            .filter(users::deleted_at.is_null()),
    )
    .set((&changes, users::updated_at.eq(diesel::dsl::now)))
    .returning(User::as_returning())
    .get_result(db)
    .await
    .api_context("Failed to update user")
}

pub async fn delete_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    acting_user_id: i64,
    id: i64,
) -> Result<(), ApiError> {
    if id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot delete system user"));
    }

    if id == acting_user_id {
        return Err(ApiError::bad_request("Cannot delete your own user"));
    }

    let affected = diesel::update(
        users::table
            .filter(users::id.eq(id))
            .filter(users::deleted_at.is_null()),
    )
    .set((
        users::deleted_at.eq(diesel::dsl::now),
        users::enabled.eq(false),
        users::updated_at.eq(diesel::dsl::now),
    ))
    .execute(db)
    .await
    .api_context("Failed to delete user")?;

    if affected == 0 {
        return Err(ApiError::not_found("User not found"));
    }

    Ok(())
}

pub async fn restore_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    id: i64,
) -> Result<User, ApiError> {
    if id == SYSTEM_USER_ID {
        return Err(ApiError::bad_request("Cannot restore system user"));
    }

    diesel::update(
        users::table
            .filter(users::id.eq(id))
            .filter(users::deleted_at.is_not_null()),
    )
    .set((
        users::deleted_at.eq::<Option<chrono::DateTime<chrono::Utc>>>(None),
        users::enabled.eq(false),
        users::updated_at.eq(diesel::dsl::now),
    ))
    .returning(User::as_returning())
    .get_result(db)
    .await
    .api_context("Failed to restore user")
}

pub async fn list_users(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    input: ListUsersInput,
) -> Result<ResourceList<User>, ApiError> {
    let page = input.page.unwrap_or(1).max(1);
    let per_page = input.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;
    let include_deleted = input.include_deleted.unwrap_or(false);

    let base_filter = || -> users::BoxedQuery<'_, diesel::pg::Pg> {
        let mut query = users::table.into_boxed();

        if !include_deleted {
            query = query.filter(users::deleted_at.is_null());
        }

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
        .api_context("Failed to count users")?;

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
        (Some(UserSortField::Enabled), Some(true)) => {
            query.order((users::enabled.desc(), users::id.asc()))
        }
        (Some(UserSortField::Enabled), _) => query.order((users::enabled.asc(), users::id.asc())),
        (Some(UserSortField::Id), Some(true)) => query.order(users::id.desc()),
        _ => query.order(users::id.asc()),
    };

    let items = query
        .limit(per_page)
        .offset(offset)
        .select(User::as_select())
        .load::<User>(db)
        .await
        .api_context("Failed to list users")?;

    Ok(ResourceList {
        total,
        page,
        per_page,
        items,
    })
}
