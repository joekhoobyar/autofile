mod support;

use autofile_api::application::document_types_metadata_types::{
    DocumentTypeMetadataTypeChangeset, DocumentTypeNewMetadataTypeInput,
    ListDocumentTypesMetadataTypesQuery, NewDocumentTypeMetadataType,
    create_document_type_metadata_type, delete_document_type_metadata_type,
    get_document_type_metadata_type, list_document_types_metadata_types,
    save_document_type_metadata_types, update_document_type_metadata_type,
};
use axum::http::StatusCode;
use diesel_async::AsyncPgConnection;
use diesel_async::pooled_connection::bb8;

use support::db::TestDatabase;
use support::fixtures::{insert_document_type, insert_metadata_type, insert_user};

const USER_ID: i64 = 9003;

async fn test_conn(test_db: &TestDatabase) -> bb8::PooledConnection<'_, AsyncPgConnection> {
    test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed")
}

async fn seed_user(db: &mut bb8::PooledConnection<'_, AsyncPgConnection>) {
    insert_user(db, USER_ID, "dtmt-tester", "dtmt-tester@example.com").await;
}

async fn seed_pair(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_type_id: i64,
    metadata_type_id: i64,
) {
    insert_document_type(
        db,
        document_type_id,
        &format!("doctype-{document_type_id}"),
        &format!("Doctype {document_type_id}"),
        USER_ID,
    )
    .await;
    insert_metadata_type(
        db,
        metadata_type_id,
        &format!("field-{metadata_type_id}"),
        &format!("Field {metadata_type_id}"),
        USER_ID,
    )
    .await;
}

#[tokio::test]
async fn create_and_get_association() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 701, 701).await;

    let created = create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 701,
            metadata_type_id: 701,
            required: true,
        },
    )
    .await
    .expect("create should succeed");

    assert_eq!(created.document_type_id, 701);
    assert_eq!(created.metadata_type_id, 701);
    assert!(created.required);

    let fetched = get_document_type_metadata_type(&mut db, 701, 701)
        .await
        .expect("get should succeed");
    assert_eq!(fetched.document_type_id, 701);
    assert_eq!(fetched.metadata_type_id, 701);
}

#[tokio::test]
async fn create_association_rejects_duplicates_and_unknown_ids() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 702, 702).await;

    create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 702,
            metadata_type_id: 702,
            required: false,
        },
    )
    .await
    .expect("first create should succeed");

    let err = create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 702,
            metadata_type_id: 702,
            required: false,
        },
    )
    .await
    .expect_err("duplicate association should fail");
    assert_eq!(err.status, StatusCode::CONFLICT);

    let err = create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 999_999,
            metadata_type_id: 702,
            required: false,
        },
    )
    .await
    .expect_err("unknown document type should fail");
    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn get_association_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = get_document_type_metadata_type(&mut db, 999_998, 999_999)
        .await
        .expect_err("missing association should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn update_association_toggles_required() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 703, 703).await;
    create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 703,
            metadata_type_id: 703,
            required: false,
        },
    )
    .await
    .expect("create should succeed");

    let updated = update_document_type_metadata_type(
        &mut db,
        703,
        703,
        DocumentTypeMetadataTypeChangeset {
            required: Some(true),
        },
    )
    .await
    .expect("update should succeed");

    assert!(updated.required);

    let err = update_document_type_metadata_type(
        &mut db,
        703,
        999_999,
        DocumentTypeMetadataTypeChangeset {
            required: Some(true),
        },
    )
    .await
    .expect_err("missing association should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn save_replaces_association_set() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 704, 704).await;
    insert_metadata_type(&mut db, 705, "field-705", "Field 705", USER_ID).await;

    let saved = save_document_type_metadata_types(
        &mut db,
        704,
        vec![
            DocumentTypeNewMetadataTypeInput {
                metadata_type_id: 704,
                required: false,
            },
            DocumentTypeNewMetadataTypeInput {
                metadata_type_id: 705,
                required: true,
            },
        ],
    )
    .await
    .expect("save should succeed");

    assert_eq!(saved.len(), 2);
    assert!(
        saved
            .iter()
            .any(|row| row.metadata_type_id == 705 && row.required)
    );

    // Saving again replaces the previous set rather than appending.
    let replaced = save_document_type_metadata_types(
        &mut db,
        704,
        vec![DocumentTypeNewMetadataTypeInput {
            metadata_type_id: 705,
            required: false,
        }],
    )
    .await
    .expect("replacement save should succeed");
    assert_eq!(replaced.len(), 1);
    assert_eq!(replaced[0].metadata_type_id, 705);

    let err = get_document_type_metadata_type(&mut db, 704, 704)
        .await
        .expect_err("replaced-away association should be gone");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn save_with_empty_input_clears_associations() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 706, 706).await;

    save_document_type_metadata_types(
        &mut db,
        706,
        vec![DocumentTypeNewMetadataTypeInput {
            metadata_type_id: 706,
            required: true,
        }],
    )
    .await
    .expect("initial save should succeed");

    let cleared = save_document_type_metadata_types(&mut db, 706, vec![])
        .await
        .expect("clearing save should succeed");
    assert!(cleared.is_empty());

    let listed = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: None,
            per_page: None,
            q: None,
            document_type_id: Some(706),
            metadata_type_id: None,
        },
    )
    .await
    .expect("list should succeed");
    assert!(listed.is_empty());
}

#[tokio::test]
async fn save_rejects_unknown_metadata_type() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 707, 707).await;

    let err = save_document_type_metadata_types(
        &mut db,
        707,
        vec![DocumentTypeNewMetadataTypeInput {
            metadata_type_id: 999_999,
            required: false,
        }],
    )
    .await
    .expect_err("unknown metadata type should fail");

    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn delete_association_removes_row() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 708, 708).await;
    create_document_type_metadata_type(
        &mut db,
        NewDocumentTypeMetadataType {
            document_type_id: 708,
            metadata_type_id: 708,
            required: false,
        },
    )
    .await
    .expect("create should succeed");

    delete_document_type_metadata_type(&mut db, 708, 708)
        .await
        .expect("delete should succeed");

    let err = get_document_type_metadata_type(&mut db, 708, 708)
        .await
        .expect_err("deleted association should be gone");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn delete_association_returns_not_found_for_missing() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;

    let err = delete_document_type_metadata_type(&mut db, 999_998, 999_999)
        .await
        .expect_err("missing association should fail");
    assert_eq!(err.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_supports_filters_search_and_pagination() {
    let test_db = TestDatabase::new().await;
    let mut db = test_conn(&test_db).await;
    seed_user(&mut db).await;
    seed_pair(&mut db, 709, 709).await;
    seed_pair(&mut db, 710, 710).await;
    for (doc_id, meta_id) in [(709, 709), (709, 710), (710, 710)] {
        create_document_type_metadata_type(
            &mut db,
            NewDocumentTypeMetadataType {
                document_type_id: doc_id,
                metadata_type_id: meta_id,
                required: false,
            },
        )
        .await
        .expect("create should succeed");
    }

    // Filter by document type.
    let by_doc = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: None,
            per_page: None,
            q: None,
            document_type_id: Some(709),
            metadata_type_id: None,
        },
    )
    .await
    .expect("doc filter should succeed");
    assert_eq!(by_doc.len(), 2);
    assert!(by_doc.iter().all(|row| row.document_type_id == 709));

    // Filter by metadata type.
    let by_meta = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: None,
            per_page: None,
            q: None,
            document_type_id: None,
            metadata_type_id: Some(710),
        },
    )
    .await
    .expect("metadata filter should succeed");
    assert!(by_meta.iter().any(|row| row.document_type_id == 709));
    assert!(by_meta.iter().any(|row| row.document_type_id == 710));

    // Search matches the joined metadata-type slug.
    let searched = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: None,
            per_page: None,
            q: Some("field-709".to_string()),
            document_type_id: None,
            metadata_type_id: None,
        },
    )
    .await
    .expect("search should succeed");
    assert!(!searched.is_empty());
    assert!(searched.iter().all(|row| row.metadata_type_id == 709));

    // Pagination applies limit/offset ordering by ids descending.
    let first_page = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: Some(1),
            per_page: Some(2),
            q: None,
            document_type_id: None,
            metadata_type_id: None,
        },
    )
    .await
    .expect("first page should succeed");
    assert_eq!(first_page.len(), 2);

    let second_page = list_document_types_metadata_types(
        &mut db,
        ListDocumentTypesMetadataTypesQuery {
            page: Some(2),
            per_page: Some(2),
            q: None,
            document_type_id: None,
            metadata_type_id: None,
        },
    )
    .await
    .expect("second page should succeed");
    assert!(!second_page.is_empty());
    for first_row in &first_page {
        assert!(
            !second_page.iter().any(|second_row| {
                second_row.document_type_id == first_row.document_type_id
                    && second_row.metadata_type_id == first_row.metadata_type_id
            }),
            "pages should not overlap"
        );
    }
}
