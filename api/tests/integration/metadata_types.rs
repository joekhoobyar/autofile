use autofile_api::application::metadata_types::{
    ListMetadataTypeValuesQuery, ListMetadataTypesQuery, MetadataTypeChangeset,
    MetadataTypeSortField, NewMetadataType, create_metadata_type, delete_metadata_type,
    get_metadata_type, get_metadata_type_by_slug, list_metadata_type_values, list_metadata_types,
};
use autofile_api::domain::metadata_types::DataType;
use axum::http::StatusCode;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8;
use serde_json::json;

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_metadata, insert_document_type, insert_user,
};

const USER_ID: i64 = 9002;

async fn test_conn(test_db: &TestDatabase) -> bb8::PooledConnection<'_, AsyncPgConnection> {
    test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed")
}

async fn seed_user(db: &mut bb8::PooledConnection<'_, AsyncPgConnection>) {
    insert_user(
        db,
        USER_ID,
        "metadata-tester",
        "metadata-tester@example.com",
    )
    .await;
}

fn new_string_type(slug: &str, name: &str) -> NewMetadataType {
    NewMetadataType {
        slug: slug.to_string(),
        name: name.to_string(),
        data_type: DataType::String,
        description: None,
        options: None,
    }
}

#[tokio::test]
async fn create_and_get_metadata_type() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(&mut db, USER_ID, new_string_type("vendor", "Vendor"))
        .await
        .expect("create should succeed");

    assert_eq!(created.slug, "vendor");
    assert_eq!(created.data_type, DataType::String);
    assert_eq!(created.created_by, USER_ID);

    let by_id = get_metadata_type(&mut db, created.id)
        .await
        .expect("get by id should succeed");
    assert_eq!(by_id, created);

    let by_slug = get_metadata_type_by_slug(&mut db, "vendor".to_string())
        .await
        .expect("get by slug should succeed");
    assert_eq!(by_slug, created);
}

#[tokio::test]
async fn create_metadata_type_validates_slug_and_uniqueness() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let err = create_metadata_type(&mut db, USER_ID, new_string_type("Bad Slug!", "Bad"))
        .await
        .expect_err("invalid slug should fail");
    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);

    create_metadata_type(&mut db, USER_ID, new_string_type("vendor", "Vendor"))
        .await
        .expect("first create should succeed");
    let err = create_metadata_type(&mut db, USER_ID, new_string_type("vendor", "Vendor Again"))
        .await
        .expect_err("duplicate slug should fail");
    assert_eq!(err.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn get_metadata_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = get_metadata_type(&mut db, 999_999)
        .await
        .expect_err("missing type should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);

    let err = get_metadata_type_by_slug(&mut db, "no-such-metadata".to_string())
        .await
        .expect_err("missing slug should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_metadata_type_updates_fields() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(&mut db, USER_ID, new_string_type("vendor", "Vendor"))
        .await
        .expect("create should succeed");

    let updated = autofile_api::application::metadata_types::update_metadata_type(
        &mut db,
        USER_ID,
        created.id,
        MetadataTypeChangeset {
            name: Some("Renamed Vendor".to_string()),
            data_type: None,
            description: Some("Updated".to_string()),
            options: None,
        },
    )
    .await
    .expect("update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "Renamed Vendor");
    assert_eq!(updated.description.as_deref(), Some("Updated"));
    assert_eq!(updated.data_type, DataType::String);
}

#[tokio::test]
async fn update_metadata_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let err = autofile_api::application::metadata_types::update_metadata_type(
        &mut db,
        USER_ID,
        999_999,
        MetadataTypeChangeset {
            name: Some("Nope".to_string()),
            data_type: None,
            description: None,
            options: None,
        },
    )
    .await
    .expect_err("missing type should fail");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn lookup_metadata_type_supports_choices_option() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(
        &mut db,
        USER_ID,
        NewMetadataType {
            slug: "category".to_string(),
            name: "Category".to_string(),
            data_type: DataType::Lookup,
            description: None,
            options: Some(json!({ "choices": ["a", "b"] })),
        },
    )
    .await
    .expect("lookup create should succeed");

    assert_eq!(created.data_type, DataType::Lookup);
    assert_eq!(created.options, Some(json!({ "choices": ["a", "b"] })));
}

#[tokio::test]
async fn list_metadata_type_values_returns_distinct_ordered_values() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(&mut db, USER_ID, new_string_type("vendor", "Vendor"))
        .await
        .expect("create should succeed");
    insert_document_type(&mut db, 601, "vendor-doctype", "Vendor Doctype", USER_ID).await;
    // One metadata row per document (primary key is document + type); the
    // duplicate "beta" across documents collapses in the distinct listing.
    for (document_id, value) in [
        (601, "beta"),
        (602, "alpha"),
        (603, "beta"),
        (604, "  "),
        (605, ""),
    ] {
        insert_document(
            &mut db,
            document_id,
            &format!("Vendor Doc {document_id}"),
            601,
            USER_ID,
        )
        .await;
        insert_document_metadata(&mut db, document_id, created.id, value, USER_ID).await;
    }

    // Blank values are excluded; duplicates collapse; ordering is ascending.
    let values = list_metadata_type_values(
        &mut db,
        created.id,
        ListMetadataTypeValuesQuery {
            q: None,
            limit: None,
        },
    )
    .await
    .expect("values should list");
    assert_eq!(values, vec!["alpha".to_string(), "beta".to_string()]);

    // Substring filtering applies.
    let filtered = list_metadata_type_values(
        &mut db,
        created.id,
        ListMetadataTypeValuesQuery {
            q: Some("alp".to_string()),
            limit: None,
        },
    )
    .await
    .expect("filtered values should list");
    assert_eq!(filtered, vec!["alpha".to_string()]);

    // Limits clamp to the requested maximum.
    let limited = list_metadata_type_values(
        &mut db,
        created.id,
        ListMetadataTypeValuesQuery {
            q: None,
            limit: Some(1),
        },
    )
    .await
    .expect("limited values should list");
    assert_eq!(limited, vec!["alpha".to_string()]);
}

#[tokio::test]
async fn list_metadata_type_values_rejects_non_string_types() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(
        &mut db,
        USER_ID,
        NewMetadataType {
            slug: "issue-date".to_string(),
            name: "Issue Date".to_string(),
            data_type: DataType::Date,
            description: None,
            options: None,
        },
    )
    .await
    .expect("date type create should succeed");

    let err = list_metadata_type_values(
        &mut db,
        created.id,
        ListMetadataTypeValuesQuery {
            q: None,
            limit: None,
        },
    )
    .await
    .expect_err("non-string values should fail");

    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn list_metadata_type_values_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = list_metadata_type_values(
        &mut db,
        999_999,
        ListMetadataTypeValuesQuery {
            q: None,
            limit: None,
        },
    )
    .await
    .expect_err("missing type should fail");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_metadata_type_blocks_in_use_types() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    // Seeded correspondent type is linked to the unspecified document type.
    let seeded = get_metadata_type_by_slug(&mut db, "correspondent".to_string())
        .await
        .expect("seeded type should exist");
    let err = delete_metadata_type(&mut db, seeded.id)
        .await
        .expect_err("linked type delete should fail");
    assert_eq!(err.status, StatusCode::CONFLICT);

    // A type referenced only by stored document metadata is also protected.
    let orphan = create_metadata_type(&mut db, USER_ID, new_string_type("orphan", "Orphan"))
        .await
        .expect("orphan create should succeed");
    insert_document_type(&mut db, 602, "orphan-doctype", "Orphan Doctype", USER_ID).await;
    insert_document(&mut db, 602, "Orphan Doc", 602, USER_ID).await;
    insert_document_metadata(&mut db, 602, orphan.id, "value", USER_ID).await;

    let err = delete_metadata_type(&mut db, orphan.id)
        .await
        .expect_err("metadata-backed delete should fail");
    assert_eq!(err.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn delete_metadata_type_deletes_unused_type() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_metadata_type(&mut db, USER_ID, new_string_type("scratch", "Scratch"))
        .await
        .expect("create should succeed");

    delete_metadata_type(&mut db, created.id)
        .await
        .expect("delete should succeed");

    let err = get_metadata_type(&mut db, created.id)
        .await
        .expect_err("deleted type should be gone");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_metadata_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = delete_metadata_type(&mut db, 999_999)
        .await
        .expect_err("missing type should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_metadata_types_supports_search_sort_and_pagination() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    create_metadata_type(
        &mut db,
        USER_ID,
        new_string_type("alpha-field", "Alpha Field"),
    )
    .await
    .expect("alpha create should succeed");
    create_metadata_type(
        &mut db,
        USER_ID,
        new_string_type("beta-field", "Beta Field"),
    )
    .await
    .expect("beta create should succeed");

    let searched = list_metadata_types(
        &mut db,
        ListMetadataTypesQuery {
            page: None,
            per_page: None,
            q: Some("alpha-field".to_string()),
            sf: None,
            sd: None,
        },
    )
    .await
    .expect("search should succeed");
    assert_eq!(searched.total, 1);
    assert_eq!(searched.items[0].slug, "alpha-field");

    let sorted = list_metadata_types(
        &mut db,
        ListMetadataTypesQuery {
            page: None,
            per_page: Some(50),
            q: None,
            sf: Some(MetadataTypeSortField::Slug),
            sd: Some(true),
        },
    )
    .await
    .expect("sorted list should succeed");
    let slugs: Vec<String> = sorted.items.iter().map(|item| item.slug.clone()).collect();
    let mut expected = slugs.clone();
    expected.sort_by(|a, b| b.cmp(a));
    assert_eq!(slugs, expected);

    let paged = list_metadata_types(
        &mut db,
        ListMetadataTypesQuery {
            page: Some(2),
            per_page: Some(2),
            q: None,
            sf: None,
            sd: None,
        },
    )
    .await
    .expect("paged list should succeed");
    assert_eq!(paged.page, 2);
    assert_eq!(paged.per_page, 2);
    assert!(paged.total >= 4);
}
