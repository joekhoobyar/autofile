use autofile_api::application::documents::{
    DocumentSortField, ExportDocumentsCsvInput, ListDocumentsQuery, export_documents_csv,
};

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_metadata, insert_document_type, insert_metadata_type,
    insert_user,
};

const USER_ID: i64 = 9301;
const DOCUMENT_TYPE_ID: i64 = 9302;
const VENDOR_METADATA_ID: i64 = 9303;
const ACCOUNT_METADATA_ID: i64 = 9304;
const FIRST_DOCUMENT_ID: i64 = 9305;
const SECOND_DOCUMENT_ID: i64 = 9306;

async fn seed_csv_export_scenario(
    db: &mut diesel_async::pooled_connection::bb8::PooledConnection<
        '_,
        diesel_async::AsyncPgConnection,
    >,
) {
    insert_user(db, USER_ID, "csv-export", "csv-export@example.com").await;
    insert_document_type(
        db,
        DOCUMENT_TYPE_ID,
        "csv-export-type",
        "CSV Export Type",
        USER_ID,
    )
    .await;
    insert_metadata_type(db, VENDOR_METADATA_ID, "csv_vendor", "Vendor", USER_ID).await;
    insert_metadata_type(db, ACCOUNT_METADATA_ID, "csv_account", "Account", USER_ID).await;
    insert_document(
        db,
        FIRST_DOCUMENT_ID,
        "CSV Alpha, Inc.",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document(
        db,
        SECOND_DOCUMENT_ID,
        "CSV Beta",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document_metadata(
        db,
        FIRST_DOCUMENT_ID,
        VENDOR_METADATA_ID,
        "Acme \"Quoted\" Co.",
        USER_ID,
    )
    .await;
    insert_document_metadata(db, FIRST_DOCUMENT_ID, ACCOUNT_METADATA_ID, "A-100", USER_ID).await;
    insert_document_metadata(
        db,
        SECOND_DOCUMENT_ID,
        VENDOR_METADATA_ID,
        "Beta Vendor",
        USER_ID,
    )
    .await;
}

#[tokio::test]
async fn exports_all_matching_documents_with_selected_metadata_order() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_csv_export_scenario(&mut db).await;

    let bytes = export_documents_csv(
        &mut db,
        ExportDocumentsCsvInput {
            search: ListDocumentsQuery {
                page: Some(1),
                per_page: Some(1),
                q: Some("CSV".to_string()),
                sf: Some(DocumentSortField::Title),
                ..Default::default()
            },
            metadata_type_ids: vec![ACCOUNT_METADATA_ID, VENDOR_METADATA_ID],
            preview_base_url: "https://autofile.example.com".to_string(),
        },
    )
    .await
    .expect("csv export should succeed");

    let csv = String::from_utf8(bytes).expect("csv should be utf8");
    assert_eq!(
        csv,
        concat!(
            "Document ID,Title,URL,Account,Vendor\n",
            "9305,\"CSV Alpha, Inc.\",https://autofile.example.com/documents/9305/preview,A-100,\"Acme \"\"Quoted\"\" Co.\"\n",
            "9306,CSV Beta,https://autofile.example.com/documents/9306/preview,,Beta Vendor\n",
        )
    );
}

#[tokio::test]
async fn export_uses_same_metadata_value_search_as_document_list() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_csv_export_scenario(&mut db).await;

    let bytes = export_documents_csv(
        &mut db,
        ExportDocumentsCsvInput {
            search: ListDocumentsQuery {
                metadata_value: Some("Beta Vendor".to_string()),
                ..Default::default()
            },
            metadata_type_ids: vec![VENDOR_METADATA_ID],
            preview_base_url: "https://autofile.example.com".to_string(),
        },
    )
    .await
    .expect("csv export should succeed");

    let csv = String::from_utf8(bytes).expect("csv should be utf8");
    assert!(csv.contains("9306,CSV Beta"));
    assert!(!csv.contains("9305,"));
}
