use autofile_api::application::documents::{ListDocumentsQuery, list_documents};
use autofile_api::domain::document_files::{SCAN_STATUS_INFECTED, SCAN_STATUS_PENDING};
use autofile_api::schema::document_files;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_file, insert_document_type, insert_user,
};

const USER_ID: i64 = 9101;
const DOCUMENT_TYPE_ID: i64 = 9102;
const PENDING_DOCUMENT_ID: i64 = 9103;
const INFECTED_DOCUMENT_ID: i64 = 9104;
const PENDING_FILE_ID: i64 = 9105;
const INFECTED_FILE_ID: i64 = 9106;

#[tokio::test]
async fn list_documents_can_narrow_by_file_scan_status() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    insert_user(&mut db, USER_ID, "scan-search", "scan-search@example.com").await;
    insert_document_type(
        &mut db,
        DOCUMENT_TYPE_ID,
        "scan-search-type",
        "Scan Search Type",
        USER_ID,
    )
    .await;
    insert_document(
        &mut db,
        PENDING_DOCUMENT_ID,
        "Scan Search Pending",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document(
        &mut db,
        INFECTED_DOCUMENT_ID,
        "Scan Search Infected",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document_file(
        &mut db,
        PENDING_FILE_ID,
        PENDING_DOCUMENT_ID,
        "pending.pdf",
        USER_ID,
    )
    .await;
    insert_document_file(
        &mut db,
        INFECTED_FILE_ID,
        INFECTED_DOCUMENT_ID,
        "infected.pdf",
        USER_ID,
    )
    .await;

    diesel::update(document_files::table.filter(document_files::id.eq(PENDING_FILE_ID)))
        .set((
            document_files::scan_status.eq(SCAN_STATUS_PENDING),
            document_files::scan_requested.eq(true),
        ))
        .execute(&mut db)
        .await
        .expect("pending file update should succeed");
    diesel::update(document_files::table.filter(document_files::id.eq(INFECTED_FILE_ID)))
        .set((
            document_files::scan_status.eq(SCAN_STATUS_INFECTED),
            document_files::scan_requested.eq(true),
        ))
        .execute(&mut db)
        .await
        .expect("infected file update should succeed");

    let results = list_documents(
        &mut db,
        ListDocumentsQuery {
            file_scan_status: Some(SCAN_STATUS_PENDING.to_string()),
            q: Some("Scan Search".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("document list should load");

    assert_eq!(results.total, 1);
    assert_eq!(results.items[0].id, PENDING_DOCUMENT_ID);
}

#[tokio::test]
async fn unknown_file_scan_status_returns_no_documents() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    insert_user(
        &mut db,
        USER_ID,
        "scan-search-unknown",
        "scan-search-unknown@example.com",
    )
    .await;
    insert_document_type(
        &mut db,
        DOCUMENT_TYPE_ID,
        "scan-search-unknown-type",
        "Scan Search Unknown Type",
        USER_ID,
    )
    .await;
    insert_document(
        &mut db,
        PENDING_DOCUMENT_ID,
        "Scan Search Unknown",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document_file(
        &mut db,
        PENDING_FILE_ID,
        PENDING_DOCUMENT_ID,
        "pending.pdf",
        USER_ID,
    )
    .await;

    let results = list_documents(
        &mut db,
        ListDocumentsQuery {
            file_scan_status: Some("not-a-status".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("document list should load");

    assert_eq!(results.total, 0);
}
