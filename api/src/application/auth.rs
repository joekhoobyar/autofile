use axum::http::StatusCode;
use bb8::PooledConnection;
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::application::app_settings::get_app_settings;
use crate::domain::users::{User, UserRole};
use crate::schema::users;
use crate::shared::auth::{hash_password, verify_password};
use crate::shared::errors::{ApiError, ApiErrorContext};

pub async fn register_user(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    username: String,
    email: String,
    display_name: String,
    password: String,
) -> Result<User, ApiError> {
    let settings = get_app_settings(db).await?;
    if !settings.allow_user_registration {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "User registration is disabled",
        ));
    }

    let pw_hash = hash_password(&password).map_err(ApiError::bad_request)?;

    let existing_email = users::table
        .filter(users::email.eq(&email))
        .select(User::as_select())
        .first::<User>(db)
        .await
        .optional()
        .api_context("Failed to register user")?;

    if let Some(existing_email) = existing_email {
        if existing_email.deleted_at.is_none() {
            return Err(ApiError::conflict("Email address is already registered"));
        }

        let username_owner = users::table
            .filter(users::username.eq(&username))
            .filter(users::id.ne(existing_email.id))
            .select(User::as_select())
            .first::<User>(db)
            .await
            .optional()
            .api_context("Failed to register user")?;

        if username_owner.is_some() {
            return Err(ApiError::conflict("Username is already taken"));
        }

        diesel::update(users::table.filter(users::id.eq(existing_email.id)))
            .set((
                users::username.eq(&username),
                users::display_name.eq(&display_name),
                users::password_hash.eq(pw_hash),
                users::password_changed_at.eq(Utc::now()),
                users::role.eq(UserRole::User),
                users::force_password_change.eq(false),
                users::enabled.eq(false),
                users::deleted_at.eq::<Option<chrono::DateTime<chrono::Utc>>>(None),
                users::updated_at.eq(diesel::dsl::now),
            ))
            .returning(User::as_returning())
            .get_result(db)
            .await
            .api_context("Failed to register user")
    } else {
        let username_owner = users::table
            .filter(users::username.eq(&username))
            .select(User::as_select())
            .first::<User>(db)
            .await
            .optional()
            .api_context("Failed to register user")?;

        if username_owner.is_some() {
            return Err(ApiError::conflict("Username is already taken"));
        }

        diesel::insert_into(users::table)
            .values((
                users::username.eq(&username),
                users::email.eq(&email),
                users::display_name.eq(&display_name),
                users::password_hash.eq(pw_hash),
                users::password_changed_at.eq(Utc::now()),
                users::role.eq(UserRole::User),
                users::enabled.eq(false),
            ))
            .returning(User::as_returning())
            .get_result(db)
            .await
            .api_context("Failed to register user")
    }
}

pub async fn find_user_for_login(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    username: &str,
) -> Result<Option<User>, ApiError> {
    users::table
        .filter(users::username.eq(username))
        .filter(users::deleted_at.is_null())
        .select(User::as_select())
        .first::<User>(db)
        .await
        .optional()
        .api_context("Failed to fetch user")
}

pub async fn find_user_for_refresh(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    uid: i64,
) -> Result<User, ApiError> {
    users::table
        .find(uid)
        .filter(users::deleted_at.is_null())
        .select(User::as_select())
        .first::<User>(db)
        .await
        .map_err(|_| ApiError::unauthorized("Invalid refresh token"))
}

pub fn validate_login_user(user: &User, password: &str) -> Result<(), ApiError> {
    let fail = || ApiError::unauthorized("Invalid credentials");

    let ok = verify_password(password, &user.password_hash).unwrap_or(false);
    if !ok || !user.enabled {
        return Err(fail());
    }

    Ok(())
}

pub fn validate_refresh_user(user: &User) -> Result<(), ApiError> {
    if !user.enabled {
        return Err(ApiError::unauthorized("Invalid refresh token"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{validate_login_user, validate_refresh_user};
    use crate::domain::users::{User, UserRole};
    use crate::shared::auth::hash_password;
    use axum::http::StatusCode;
    use chrono::Utc;

    fn test_user(enabled: bool) -> User {
        User {
            id: 42,
            username: "test-user".to_string(),
            email: "test-user@example.com".to_string(),
            display_name: "Test User".to_string(),
            password_hash: hash_password("long-enough-password").expect("hash should succeed"),
            password_changed_at: Utc::now(),
            role: UserRole::User,
            force_password_change: false,
            enabled,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
        }
    }

    #[test]
    fn disabled_user_cannot_login() {
        let err = validate_login_user(&test_user(false), "long-enough-password")
            .expect_err("disabled user should not login");

        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.message, "Invalid credentials");
    }

    #[test]
    fn disabled_user_cannot_refresh() {
        let err =
            validate_refresh_user(&test_user(false)).expect_err("disabled user should not refresh");

        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.message, "Invalid refresh token");
    }

    #[test]
    fn enabled_user_can_login_and_refresh() {
        let user = test_user(true);

        validate_login_user(&user, "long-enough-password").expect("enabled user should login");
        validate_refresh_user(&user).expect("enabled user should refresh");
    }
}
