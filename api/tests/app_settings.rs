mod support;

use autofile_api::api::auth::{RegisterRequest, register};
use autofile_api::api::public_settings::get_public_settings;
use autofile_api::application::app_settings::{
    UpdateAppSettingsInput, get_app_settings, update_app_settings,
};
use autofile_api::application::users::delete_user;
use autofile_api::schema::users;
use autofile_api::shared::extractors::DbConn;
use axum::Json;
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use support::db::TestDatabase;

#[tokio::test]
async fn settings_default_to_allowing_user_registration() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let settings = get_app_settings(&mut db)
        .await
        .expect("settings should load");

    assert!(settings.allow_user_registration);
}

#[tokio::test]
async fn public_settings_expose_registration_flag() {
    let test_db = TestDatabase::new().await;
    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");

    let public = get_public_settings(DbConn(conn))
        .await
        .expect("public settings should load")
        .0;

    assert!(public.allow_user_registration);

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    update_app_settings(
        &mut db,
        UpdateAppSettingsInput {
            allow_user_registration: false,
        },
    )
    .await
    .expect("settings should update");
    drop(db);

    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");
    let public = get_public_settings(DbConn(conn))
        .await
        .expect("public settings should load")
        .0;

    assert!(!public.allow_user_registration);
}

#[tokio::test]
async fn registered_users_are_disabled_by_default() {
    let test_db = TestDatabase::new().await;
    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");

    let user = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-disabled-by-default".to_string(),
            email: "settings-register-disabled-by-default@example.com".to_string(),
            display_name: "Disabled By Default".to_string(),
            password: "long-enough-password".to_string(),
        }),
    )
    .await
    .expect("registration should succeed")
    .0;

    assert!(!user.enabled);

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    let persisted_enabled = users::table
        .filter(users::username.eq("settings-register-disabled-by-default"))
        .select(users::enabled)
        .first::<bool>(&mut db)
        .await
        .expect("user should load");

    assert!(!persisted_enabled);
}

#[tokio::test]
async fn register_undeletes_matching_email_with_available_username() {
    let test_db = TestDatabase::new().await;
    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");

    let original = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-undelete-old".to_string(),
            email: "settings-register-undelete@example.com".to_string(),
            display_name: "Deleted User".to_string(),
            password: "long-enough-password".to_string(),
        }),
    )
    .await
    .expect("registration should succeed")
    .0;

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    delete_user(&mut db, 1, original.id)
        .await
        .expect("delete should succeed");
    drop(db);

    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");
    let restored = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-undelete-new".to_string(),
            email: "settings-register-undelete@example.com".to_string(),
            display_name: "Restored User".to_string(),
            password: "another-long-password".to_string(),
        }),
    )
    .await
    .expect("registration should undelete matching email")
    .0;

    assert_eq!(restored.id, original.id);
    assert_eq!(restored.username, "settings-register-undelete-new");
    assert_eq!(restored.display_name, "Restored User");
    assert!(!restored.enabled);
    assert!(restored.deleted_at.is_none());
}

#[tokio::test]
async fn register_rejects_taken_username_with_different_email() {
    let test_db = TestDatabase::new().await;
    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");

    let _ = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-taken-username".to_string(),
            email: "settings-register-taken-username@example.com".to_string(),
            display_name: "Original User".to_string(),
            password: "long-enough-password".to_string(),
        }),
    )
    .await
    .expect("registration should succeed");

    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");
    let err = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-taken-username".to_string(),
            email: "settings-register-new-email@example.com".to_string(),
            display_name: "New User".to_string(),
            password: "another-long-password".to_string(),
        }),
    )
    .await
    .expect_err("taken username should fail");

    assert_eq!(err.status, StatusCode::CONFLICT);
    assert_eq!(err.message, "Username is already taken");
}

#[tokio::test]
async fn disabled_registration_rejects_register_request() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    update_app_settings(
        &mut db,
        UpdateAppSettingsInput {
            allow_user_registration: false,
        },
    )
    .await
    .expect("settings should update");
    drop(db);

    let conn = test_db
        .pool
        .get_owned()
        .await
        .expect("db connection should succeed");
    let err = register(
        DbConn(conn),
        Json(RegisterRequest {
            username: "settings-register-disabled".to_string(),
            email: "settings-register-disabled@example.com".to_string(),
            display_name: "Registration Disabled".to_string(),
            password: "long-enough-password".to_string(),
        }),
    )
    .await
    .expect_err("disabled registration should fail");

    assert_eq!(err.status, StatusCode::FORBIDDEN);
    assert_eq!(err.message, "User registration is disabled");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    let count = users::table
        .filter(users::username.eq("settings-register-disabled"))
        .count()
        .get_result::<i64>(&mut db)
        .await
        .expect("user count should load");

    assert_eq!(count, 0);
}
