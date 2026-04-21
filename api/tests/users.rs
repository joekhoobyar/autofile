mod support;

use autofile_api::application::users::{
    ListUsersInput, UpdateUserInput, UserSortField, delete_user, get_user_by_id, get_user_by_username,
    list_users, update_user,
};
use autofile_api::domain::users::User;
use autofile_api::schema::users;
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
async fn get_user_by_id_returns_user() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 101, "users-test-alpha", "users-test-alpha@example.com").await;

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
    insert_user(&mut db, 102, "users-test-beta", "users-test-beta@example.com").await;

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
        103,
        UpdateUserInput {
            email: Some("users-test-gamma-new@example.com".to_string()),
            display_name: Some("Updated User".to_string()),
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

    delete_user(&mut db, 104).await.expect("delete should succeed");

    let err = get_user_by_id(&mut db, 104)
        .await
        .expect_err("deleted user should not exist");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_users_applies_pagination_search_and_sort() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 210, "users-test-charlie", "users-test-charlie@example.com").await;
    insert_user(&mut db, 220, "users-test-alpha", "users-test-alpha@example.com").await;
    insert_user(&mut db, 230, "users-test-bravo", "users-test-bravo@example.com").await;

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
        230,
        UpdateUserInput {
            email: None,
            display_name: Some("Captain Bravo".to_string()),
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
