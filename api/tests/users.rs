mod support;

use autofile_api::application::users::{
    ChangePasswordInput, ListUsersInput, UpdateProfileInput, UpdateUserInput, UserSortField,
    change_password, delete_user, get_profile, get_user_by_id, get_user_by_username, list_users,
    update_profile, update_user,
};
use autofile_api::domain::users::{User, UserRole};
use autofile_api::schema::users;
use autofile_api::shared::auth::verify_password;
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;
use serde_json::json;

use support::db::TestDatabase;
use support::fixtures::insert_user;

async fn load_user(db: &mut bb8::PooledConnection<'_, AsyncPgConnection>, id: i64) -> User {
    users::table
        .find(id)
        .select(User::as_select())
        .first::<User>(db)
        .await
        .expect("user should load")
}

#[tokio::test]
async fn migrations_seed_default_admin_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let admin = get_user_by_username(&mut db, "admin".to_string())
        .await
        .expect("default admin should exist");

    assert_eq!(admin.email, "admin@example.com");
    assert_eq!(admin.display_name, "Admin");
    assert_eq!(admin.role, UserRole::Admin);
    assert!(admin.enabled);
    assert!(admin.force_password_change);
    assert!(
        verify_password("admin123!", &admin.password_hash).expect("hash should parse"),
        "default admin password should verify"
    );
}

#[tokio::test]
async fn inserted_users_default_to_enabled() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        108,
        "users-test-enabled-default",
        "users-test-enabled-default@example.com",
    )
    .await;

    let user = get_user_by_id(&mut db, 108)
        .await
        .expect("user should load");

    assert!(user.enabled);
}

#[tokio::test]
async fn get_user_by_id_returns_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        101,
        "users-test-alpha",
        "users-test-alpha@example.com",
    )
    .await;

    let user = get_user_by_id(&mut db, 101)
        .await
        .expect("get by id should succeed");

    assert_eq!(user.id, 101);
    assert_eq!(user.username, "users-test-alpha");
    assert_eq!(user.email, "users-test-alpha@example.com");
}

#[tokio::test]
async fn get_user_by_id_returns_not_found_for_missing_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let err = get_user_by_id(&mut db, 999)
        .await
        .expect_err("missing user should fail");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn get_user_by_username_returns_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        102,
        "users-test-beta",
        "users-test-beta@example.com",
    )
    .await;

    let user = get_user_by_username(&mut db, "users-test-beta".to_string())
        .await
        .expect("get by username should succeed");

    assert_eq!(user.id, 102);
    assert_eq!(user.username, "users-test-beta");
}

#[tokio::test]
async fn update_user_updates_fields_and_preserves_password_data() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        103,
        "users-test-gamma",
        "users-test-gamma@example.com",
    )
    .await;

    let before = load_user(&mut db, 103).await;

    let updated = update_user(
        &mut db,
        1,
        103,
        UpdateUserInput {
            email: Some("users-test-gamma-new@example.com".to_string()),
            display_name: Some("Updated User".to_string()),
            role: None,
            force_password_change: None,
            enabled: None,
        },
    )
    .await
    .expect("update should succeed");

    let after = load_user(&mut db, 103).await;

    assert_eq!(updated.email, "users-test-gamma-new@example.com");
    assert_eq!(updated.display_name, "Updated User");
    assert_eq!(before.password_hash, after.password_hash);
    assert_eq!(before.password_changed_at, after.password_changed_at);
}

#[tokio::test]
async fn update_user_updates_role() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        105,
        "users-test-role",
        "users-test-role@example.com",
    )
    .await;

    let promoted = update_user(
        &mut db,
        1,
        105,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: Some(UserRole::Admin),
            force_password_change: Some(true),
            enabled: None,
        },
    )
    .await
    .expect("role update should succeed");
    assert_eq!(promoted.role, UserRole::Admin);
    assert!(promoted.force_password_change);

    let demoted = update_user(
        &mut db,
        1,
        105,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: Some(UserRole::User),
            force_password_change: Some(false),
            enabled: None,
        },
    )
    .await
    .expect("role update should succeed");
    assert_eq!(demoted.role, UserRole::User);
    assert!(!demoted.force_password_change);
}

#[tokio::test]
async fn update_user_updates_enabled_status() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        109,
        "users-test-enabled-update",
        "users-test-enabled-update@example.com",
    )
    .await;

    let disabled = update_user(
        &mut db,
        1,
        109,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: None,
            force_password_change: None,
            enabled: Some(false),
        },
    )
    .await
    .expect("enabled update should succeed");
    assert!(!disabled.enabled);

    let enabled = update_user(
        &mut db,
        1,
        109,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: None,
            force_password_change: None,
            enabled: Some(true),
        },
    )
    .await
    .expect("enabled update should succeed");
    assert!(enabled.enabled);
}

#[tokio::test]
async fn update_user_rejects_self_downgrade() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        106,
        "users-test-self-role",
        "users-test-self-role@example.com",
    )
    .await;

    let err = update_user(
        &mut db,
        106,
        106,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: Some(UserRole::User),
            force_password_change: None,
            enabled: None,
        },
    )
    .await
    .expect_err("self downgrade should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let user = load_user(&mut db, 106).await;
    assert_eq!(user.role, UserRole::Admin);
}

#[tokio::test]
async fn update_user_rejects_self_disable() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        110,
        "users-test-self-disable",
        "users-test-self-disable@example.com",
    )
    .await;

    let err = update_user(
        &mut db,
        110,
        110,
        UpdateUserInput {
            email: None,
            display_name: None,
            role: None,
            force_password_change: None,
            enabled: Some(false),
        },
    )
    .await
    .expect_err("self disable should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let user = load_user(&mut db, 110).await;
    assert!(user.enabled);
}

#[tokio::test]
async fn get_profile_returns_current_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        107,
        "users-test-profile",
        "users-test-profile@example.com",
    )
    .await;

    let profile = get_profile(&mut db, 107)
        .await
        .expect("profile should load");

    assert_eq!(profile.id, 107);
    assert_eq!(profile.username, "users-test-profile");
}

#[tokio::test]
async fn update_profile_updates_only_profile_fields() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        108,
        "users-test-profile-update",
        "users-test-profile-update@example.com",
    )
    .await;

    let before = load_user(&mut db, 108).await;
    let updated = update_profile(
        &mut db,
        108,
        UpdateProfileInput {
            email: Some("users-test-profile-new@example.com".to_string()),
            display_name: Some("Profile User".to_string()),
        },
    )
    .await
    .expect("profile update should succeed");

    assert_eq!(updated.email, "users-test-profile-new@example.com");
    assert_eq!(updated.display_name, "Profile User");
    assert_eq!(updated.role, before.role);
    assert_eq!(updated.password_hash, before.password_hash);
    assert_eq!(updated.password_changed_at, before.password_changed_at);
}

#[tokio::test]
async fn change_password_updates_password_hash_and_timestamp() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        109,
        "users-test-password",
        "users-test-password@example.com",
    )
    .await;

    let before = load_user(&mut db, 109).await;
    let updated = change_password(
        &mut db,
        109,
        ChangePasswordInput {
            new_password: "new-password-123".to_string(),
        },
    )
    .await
    .expect("password change should succeed");

    assert_ne!(updated.password_hash, before.password_hash);
    assert!(updated.password_changed_at >= before.password_changed_at);
    assert!(!updated.force_password_change);
    assert!(
        verify_password("new-password-123", &updated.password_hash).expect("hash should parse"),
        "new password should verify"
    );
}

#[tokio::test]
async fn change_password_rejects_short_password() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        110,
        "users-test-short-password",
        "users-test-short-password@example.com",
    )
    .await;

    let err = change_password(
        &mut db,
        110,
        ChangePasswordInput {
            new_password: "too-short".to_string(),
        },
    )
    .await
    .expect_err("short password should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_user_removes_row_and_future_reads_fail() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        104,
        "users-test-delta",
        "users-test-delta@example.com",
    )
    .await;

    delete_user(&mut db, 1, 104)
        .await
        .expect("delete should succeed");

    let err = get_user_by_id(&mut db, 104)
        .await
        .expect_err("deleted user should not exist");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_user_rejects_system_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let before = get_user_by_id(&mut db, 1)
        .await
        .expect("system user should exist");

    let err = update_user(
        &mut db,
        1,
        1,
        UpdateUserInput {
            email: Some("new-system-email@example.com".to_string()),
            display_name: Some("Should Not Change".to_string()),
            role: Some(UserRole::User),
            force_password_change: Some(true),
            enabled: Some(false),
        },
    )
    .await
    .expect_err("system user update should fail");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let after = get_user_by_id(&mut db, 1)
        .await
        .expect("system user should still exist");
    assert_eq!(before.email, after.email);
    assert_eq!(before.display_name, after.display_name);
}

#[tokio::test]
async fn delete_user_rejects_system_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let err = delete_user(&mut db, 2, 1)
        .await
        .expect_err("system user delete should fail");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let system_user = get_user_by_id(&mut db, 1)
        .await
        .expect("system user should still exist");
    assert_eq!(system_user.id, 1);
}

#[tokio::test]
async fn delete_user_rejects_self_delete() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        107,
        "users-test-self-delete",
        "users-test-self-delete@example.com",
    )
    .await;

    let err = delete_user(&mut db, 107, 107)
        .await
        .expect_err("self delete should fail");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);

    let user = get_user_by_id(&mut db, 107)
        .await
        .expect("user should still exist");
    assert_eq!(user.id, 107);
}

#[tokio::test]
async fn list_users_applies_pagination_search_and_sort() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(
        &mut db,
        210,
        "users-test-charlie",
        "users-test-charlie@example.com",
    )
    .await;
    insert_user(
        &mut db,
        220,
        "users-test-alpha",
        "users-test-alpha@example.com",
    )
    .await;
    insert_user(
        &mut db,
        230,
        "users-test-bravo",
        "users-test-bravo@example.com",
    )
    .await;

    let page_one = list_users(
        &mut db,
        ListUsersInput {
            page: Some(1),
            per_page: Some(2),
            q: Some("users-test".to_string()),
            sf: Some(UserSortField::Username),
            sd: Some(false),
        },
    )
    .await
    .expect("list should succeed");

    assert_eq!(page_one.total, 3);
    assert_eq!(page_one.page, 1);
    assert_eq!(page_one.per_page, 2);
    assert_eq!(page_one.items.len(), 2);
    assert_eq!(page_one.items[0].username, "users-test-alpha");
    assert_eq!(page_one.items[1].username, "users-test-bravo");

    let username_filtered = list_users(
        &mut db,
        ListUsersInput {
            page: None,
            per_page: None,
            q: Some("users-test-char".to_string()),
            sf: None,
            sd: None,
        },
    )
    .await
    .expect("filtered list should succeed");
    assert_eq!(username_filtered.items.len(), 1);
    assert_eq!(username_filtered.items[0].username, "users-test-charlie");

    update_user(
        &mut db,
        1,
        230,
        UpdateUserInput {
            email: None,
            display_name: Some("Captain Bravo".to_string()),
            role: None,
            force_password_change: None,
            enabled: None,
        },
    )
    .await
    .expect("display name update should succeed");

    let display_name_filtered = list_users(
        &mut db,
        ListUsersInput {
            page: None,
            per_page: None,
            q: Some("captain".to_string()),
            sf: None,
            sd: None,
        },
    )
    .await
    .expect("display name filter should succeed");
    assert_eq!(display_name_filtered.items.len(), 1);
    assert_eq!(display_name_filtered.items[0].id, 230);

    let email_filtered = list_users(
        &mut db,
        ListUsersInput {
            page: None,
            per_page: None,
            q: Some("users-test-alpha@example".to_string()),
            sf: None,
            sd: None,
        },
    )
    .await
    .expect("email filter should succeed");
    assert_eq!(email_filtered.items.len(), 1);
    assert_eq!(email_filtered.items[0].id, 220);
}

#[test]
fn update_user_input_rejects_unknown_password_fields() {
    let payload = json!({
        "email": "new@example.com",
        "password": "new-secret"
    });

    let err = serde_json::from_value::<UpdateUserInput>(payload)
        .expect_err("password field should be rejected");

    assert!(
        err.to_string().contains("unknown field `password`"),
        "unexpected deserialize error: {err}"
    );
}

#[test]
fn update_profile_input_rejects_role_field() {
    let payload = json!({
        "email": "new@example.com",
        "role": "admin"
    });

    let err = serde_json::from_value::<UpdateProfileInput>(payload)
        .expect_err("role field should be rejected");

    assert!(
        err.to_string().contains("unknown field `role`"),
        "unexpected deserialize error: {err}"
    );
}
