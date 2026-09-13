mod support;

use autofile_api::application::document_types::{
    DocumentTypeChangeset, ListDocumentTypesQuery, NewDocumentType, create_document_type,
    delete_document_type, get_document_type, get_document_type_by_slug, list_document_types,
    update_document_type,
};
use autofile_api::schema::documents;
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use support::db::TestDatabase;
use support::fixtures::{insert_document, insert_user};

const USER_ID: i64 = 9001;

async fn test_conn(test_db: &TestDatabase) -> bb8::PooledConnection<'_, AsyncPgConnection> {
    test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed")
}

async fn seed_user(db: &mut bb8::PooledConnection<'_, AsyncPgConnection>) {
    insert_user(db, USER_ID, "doctype-tester", "doctype-tester@example.com").await;
}

fn new_type(slug: &str, name: &str) -> NewDocumentType {
    NewDocumentType {
        slug: slug.to_string(),
        name: name.to_string(),
        description: None,
    }
}

fn list_query(q: Option<&str>, page: Option<i64>, per_page: Option<i64>) -> ListDocumentTypesQuery {
    ListDocumentTypesQuery {
        page,
        per_page,
        q: q.map(str::to_string),
        sf: None,
        sd: None,
    }
}

async fn document_type_id_of_document(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_id: i64,
) -> i64 {
    documents::table
        .find(document_id)
        .select(documents::document_type_id)
        .first::<i64>(db)
        .await
        .expect("document should load")
}

#[tokio::test]
async fn create_document_type_creates_row() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_document_type(&mut db, USER_ID, new_type("invoice", "Invoice"))
        .await
        .expect("create should succeed");

    assert_eq!(created.slug, "invoice");
    assert_eq!(created.name, "Invoice");
    assert_eq!(created.created_by, USER_ID);
    assert_eq!(created.updated_by, USER_ID);

    let fetched = get_document_type(&mut db, created.id)
        .await
        .expect("get by id should succeed");
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn create_document_type_rejects_invalid_slug() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let err = create_document_type(&mut db, USER_ID, new_type("Invalid Slug!", "Bad"))
        .await
        .expect_err("invalid slug should fail");

    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_document_type_rejects_duplicate_slug() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    create_document_type(&mut db, USER_ID, new_type("invoice", "Invoice"))
        .await
        .expect("first create should succeed");

    let err = create_document_type(&mut db, USER_ID, new_type("invoice", "Invoice Again"))
        .await
        .expect_err("duplicate slug should fail");

    assert_eq!(err.status, StatusCode::CONFLICT);
}

#[tokio::test]
async fn get_document_type_by_slug_returns_row() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_document_type(&mut db, USER_ID, new_type("receipt", "Receipt"))
        .await
        .expect("create should succeed");

    let by_slug = get_document_type_by_slug(&mut db, "receipt".to_string())
        .await
        .expect("get by slug should succeed");
    assert_eq!(by_slug, created);
}

#[tokio::test]
async fn get_document_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = get_document_type(&mut db, 999_999)
        .await
        .expect_err("missing type should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);

    let err = get_document_type_by_slug(&mut db, "no-such-type".to_string())
        .await
        .expect_err("missing slug should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_document_type_updates_fields() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_document_type(&mut db, USER_ID, new_type("invoice", "Invoice"))
        .await
        .expect("create should succeed");

    let updated = update_document_type(
        &mut db,
        USER_ID,
        created.id,
        DocumentTypeChangeset {
            name: Some("Renamed Invoice".to_string()),
            description: Some("Updated description".to_string()),
        },
    )
    .await
    .expect("update should succeed");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.slug, "invoice");
    assert_eq!(updated.name, "Renamed Invoice");
    assert_eq!(updated.description.as_deref(), Some("Updated description"));
}

#[tokio::test]
async fn update_document_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let err = update_document_type(
        &mut db,
        USER_ID,
        999_999,
        DocumentTypeChangeset {
            name: Some("Nope".to_string()),
            description: None,
        },
    )
    .await
    .expect_err("missing type should fail");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_document_type_reassigns_documents_to_default() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let created = create_document_type(&mut db, USER_ID, new_type("invoice", "Invoice"))
        .await
        .expect("create should succeed");
    insert_document(&mut db, 501, "Doc With Type", created.id, USER_ID).await;

    delete_document_type(&mut db, USER_ID, created.id)
        .await
        .expect("delete should succeed");

    assert_eq!(document_type_id_of_document(&mut db, 501).await, 1);

    let err = get_document_type(&mut db, created.id)
        .await
        .expect_err("deleted type should be gone");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_document_type_rejects_default_type() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = delete_document_type(&mut db, USER_ID, 1)
        .await
        .expect_err("default type delete should fail");

    assert_eq!(err.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn delete_document_type_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = delete_document_type(&mut db, USER_ID, 999_999)
        .await
        .expect_err("missing type should fail");

    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_document_types_supports_search_pagination_and_counts() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;

    let alpha = create_document_type(&mut db, USER_ID, new_type("alpha-type", "Alpha Type"))
        .await
        .expect("create alpha should succeed");
    create_document_type(&mut db, USER_ID, new_type("beta-type", "Beta Type"))
        .await
        .expect("create beta should succeed");
    create_document_type(&mut db, USER_ID, new_type("gamma-type", "Gamma Type"))
        .await
        .expect("create gamma should succeed");
    insert_document(&mut db, 502, "Alpha Doc", alpha.id, USER_ID).await;

    // Search narrows to the matching row and reports its document count.
    let searched = list_document_types(&mut db, list_query(Some("alpha"), None, None))
        .await
        .expect("search should succeed");
    assert_eq!(searched.total, 1);
    assert_eq!(searched.items.len(), 1);
    assert_eq!(searched.items[0].slug, "alpha-type");
    assert_eq!(searched.items[0].document_count, 1);

    // Pagination pages through the full set (plus the seeded unspecified type).
    let page_one = list_document_types(&mut db, list_query(None, Some(1), Some(2)))
        .await
        .expect("page one should succeed");
    assert_eq!(page_one.page, 1);
    assert_eq!(page_one.per_page, 2);
    assert!(page_one.total >= 4);
    assert_eq!(page_one.items.len(), 2);

    let page_two = list_document_types(&mut db, list_query(None, Some(2), Some(2)))
        .await
        .expect("page two should succeed");
    assert_eq!(page_two.page, 2);
    assert!(!page_two.items.is_empty());

    // Sorting by name descending puts gamma first.
    let sorted = list_document_types(
        &mut db,
        ListDocumentTypesQuery {
            page: None,
            per_page: Some(50),
            q: None,
            sf: Some(autofile_api::application::document_types::DocumentTypeSortField::Name),
            sd: Some(true),
        },
    )
    .await
    .expect("sorted list should succeed");
    let names: Vec<String> = sorted.items.iter().map(|item| item.name.clone()).collect();
    let mut expected = names.clone();
    expected.sort_by(|a, b| b.cmp(a));
    assert_eq!(names, expected);
}
