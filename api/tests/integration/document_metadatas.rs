use autofile_api::application::document_metadatas::{
    NewDocumentMetadata, document_metadatas_upsert, get_document_metadata,
};
use autofile_api::application::documents::{ListDocumentsQuery, list_documents};
use autofile_api::schema::document_metadatas;
use axum::http::StatusCode;
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_metadata, insert_document_type, insert_metadata_type,
    insert_metadata_type_with_data_type, insert_user, link_document_type_metadata,
};

const USER_ID: i64 = 9201;
const DOCUMENT_TYPE_ID: i64 = 9202;
const STRING_METADATA_ID: i64 = 9203;
const DATE_METADATA_ID: i64 = 9204;
const STRING_DOCUMENT_ID: i64 = 9205;
const DATE_DOCUMENT_ID: i64 = 9206;

async fn seed_metadata_search_scenario(
    db: &mut bb8::PooledConnection<'_, diesel_async::AsyncPgConnection>,
) {
    insert_user(db, USER_ID, "metadata-docs", "metadata-docs@example.com").await;
    insert_document_type(
        db,
        DOCUMENT_TYPE_ID,
        "metadata-docs-type",
        "Metadata Docs Type",
        USER_ID,
    )
    .await;
    insert_metadata_type(db, STRING_METADATA_ID, "tmv_vendor", "TMV Vendor", USER_ID).await;
    insert_metadata_type_with_data_type(
        db,
        DATE_METADATA_ID,
        "tmv_issue_date",
        "TMV Issue Date",
        "date",
        USER_ID,
    )
    .await;
    link_document_type_metadata(db, DOCUMENT_TYPE_ID, STRING_METADATA_ID).await;
    link_document_type_metadata(db, DOCUMENT_TYPE_ID, DATE_METADATA_ID).await;
    insert_document(
        db,
        STRING_DOCUMENT_ID,
        "String Metadata Doc",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document(
        db,
        DATE_DOCUMENT_ID,
        "Date Metadata Doc",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
}

#[tokio::test]
async fn saves_and_reads_string_and_date_metadata_rows() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_metadata_search_scenario(&mut db).await;

    document_metadatas_upsert(
        USER_ID,
        &mut db,
        STRING_DOCUMENT_ID,
        vec![
            NewDocumentMetadata {
                metadata_type_id: STRING_METADATA_ID,
                string_value: Some("Acme".to_string()),
                number_value: None,
                date_value: None,
            },
            NewDocumentMetadata {
                metadata_type_id: DATE_METADATA_ID,
                string_value: None,
                number_value: None,
                date_value: Some("2026-08-25".to_string()),
            },
        ],
    )
    .await
    .expect("metadata upsert should succeed");

    let string_row = get_document_metadata(&mut db, STRING_DOCUMENT_ID, STRING_METADATA_ID)
        .await
        .expect("string metadata should load");
    assert_eq!(string_row.string_value.as_deref(), Some("Acme"));
    assert!(string_row.number_value.is_none());
    assert!(string_row.date_value.is_none());

    let date_row = get_document_metadata(&mut db, STRING_DOCUMENT_ID, DATE_METADATA_ID)
        .await
        .expect("date metadata should load");
    assert!(date_row.string_value.is_none());
    assert!(date_row.number_value.is_none());
    assert_eq!(date_row.date_value, NaiveDate::from_ymd_opt(2026, 8, 25));
}

#[tokio::test]
async fn rejects_multiple_typed_values_and_number_values() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_metadata_search_scenario(&mut db).await;

    let err = document_metadatas_upsert(
        USER_ID,
        &mut db,
        STRING_DOCUMENT_ID,
        vec![NewDocumentMetadata {
            metadata_type_id: STRING_METADATA_ID,
            string_value: Some("Acme".to_string()),
            number_value: None,
            date_value: Some("2026-08-25".to_string()),
        }],
    )
    .await
    .expect_err("multiple typed values should fail");
    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);

    let err = document_metadatas_upsert(
        USER_ID,
        &mut db,
        STRING_DOCUMENT_ID,
        vec![NewDocumentMetadata {
            metadata_type_id: STRING_METADATA_ID,
            string_value: None,
            number_value: Some(bigdecimal::BigDecimal::from(5)),
            date_value: None,
        }],
    )
    .await
    .expect_err("number values should fail");
    assert_eq!(err.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn metadata_value_search_only_matches_string_values() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_metadata_search_scenario(&mut db).await;
    insert_document_metadata(
        &mut db,
        STRING_DOCUMENT_ID,
        STRING_METADATA_ID,
        "2026-08-25 Acme",
        USER_ID,
    )
    .await;
    document_metadatas_upsert(
        USER_ID,
        &mut db,
        DATE_DOCUMENT_ID,
        vec![NewDocumentMetadata {
            metadata_type_id: DATE_METADATA_ID,
            string_value: None,
            number_value: None,
            date_value: Some("2026-08-25".to_string()),
        }],
    )
    .await
    .expect("date metadata upsert should succeed");

    let results = list_documents(
        &mut db,
        ListDocumentsQuery {
            metadata_value: Some("2026-08-25".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("document list should load");

    let ids: Vec<i64> = results.items.into_iter().map(|doc| doc.id).collect();
    assert_eq!(ids, vec![STRING_DOCUMENT_ID]);
}

#[tokio::test]
async fn metadata_type_filter_matches_date_metadata_rows() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_metadata_search_scenario(&mut db).await;
    document_metadatas_upsert(
        USER_ID,
        &mut db,
        DATE_DOCUMENT_ID,
        vec![NewDocumentMetadata {
            metadata_type_id: DATE_METADATA_ID,
            string_value: None,
            number_value: None,
            date_value: Some("2026-08-25".to_string()),
        }],
    )
    .await
    .expect("date metadata upsert should succeed");

    let results = list_documents(
        &mut db,
        ListDocumentsQuery {
            metadata_type_id: Some(DATE_METADATA_ID),
            ..Default::default()
        },
    )
    .await
    .expect("document list should load");

    assert_eq!(results.total, 1);
    assert_eq!(results.items[0].id, DATE_DOCUMENT_ID);
    assert_eq!(
        results.items[0].metadata.get("tmv_issue_date"),
        Some(&"2026-08-25".to_string())
    );
}

#[tokio::test]
async fn fixture_inserts_string_metadata_values() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_metadata_search_scenario(&mut db).await;
    insert_document_metadata(
        &mut db,
        STRING_DOCUMENT_ID,
        STRING_METADATA_ID,
        "fixture-value",
        USER_ID,
    )
    .await;

    let row: (Option<String>, Option<NaiveDate>) = document_metadatas::table
        .filter(document_metadatas::document_id.eq(STRING_DOCUMENT_ID))
        .filter(document_metadatas::metadata_type_id.eq(STRING_METADATA_ID))
        .select((
            document_metadatas::string_value,
            document_metadatas::date_value,
        ))
        .first(&mut db)
        .await
        .expect("metadata row should load");

    assert_eq!(row, (Some("fixture-value".to_string()), None));
}
