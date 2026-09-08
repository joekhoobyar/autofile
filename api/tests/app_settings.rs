mod support;

use autofile_api::api::auth::{RegisterRequest, register};
use autofile_api::application::app_settings::{
    UpdateAppSettingsInput, get_app_settings, update_app_settings,
};
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
