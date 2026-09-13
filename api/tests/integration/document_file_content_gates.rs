use autofile_api::application::document_files::{
    get_document_file_download_metadata, get_document_file_page_image_key,
    get_document_file_thumbnail_metadata, list_document_file_ocr_pages, list_document_file_pages,
};
use autofile_api::application::documents::get_document_thumbnail_metadata;
use autofile_api::domain::document_files::SCAN_STATUS_PENDING;
use autofile_api::schema::{document_file_ocr_pages, document_files, documents};
use autofile_api::shared::errors::ApiError;
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_file, insert_document_file_page, insert_document_type,
    insert_user,
};

const USER_ID: i64 = 7001;
const DOCUMENT_TYPE_ID: i64 = 7002;
const DOCUMENT_ID: i64 = 7003;
const DOCUMENT_FILE_ID: i64 = 7004;

async fn seed_pending_file(
    db: &mut diesel_async::pooled_connection::bb8::PooledConnection<
        '_,
        diesel_async::AsyncPgConnection,
    >,
) {
    insert_user(db, USER_ID, "gate-user", "gate-user@example.com").await;
    insert_document_type(db, DOCUMENT_TYPE_ID, "gate-type", "Gate Type", USER_ID).await;
    insert_document(db, DOCUMENT_ID, "Gate Document", DOCUMENT_TYPE_ID, USER_ID).await;
    insert_document_file(db, DOCUMENT_FILE_ID, DOCUMENT_ID, "gate.pdf", USER_ID).await;
    insert_document_file_page(db, DOCUMENT_FILE_ID, 1, "unsafe text").await;
    diesel::insert_into(document_file_ocr_pages::table)
        .values((
            document_file_ocr_pages::document_file_id.eq(DOCUMENT_FILE_ID),
            document_file_ocr_pages::page_number.eq(1_i32),
            document_file_ocr_pages::ocr_content.eq::<Option<String>>(Some("unsafe ocr".into())),
        ))
        .execute(db)
        .await
        .expect("ocr page insert should succeed");
    diesel::update(document_files::table.filter(document_files::id.eq(DOCUMENT_FILE_ID)))
        .set((
            document_files::scan_status.eq(SCAN_STATUS_PENDING),
            document_files::scan_requested.eq(true),
        ))
        .execute(db)
        .await
        .expect("scan status update should succeed");
    diesel::update(documents::table.filter(documents::id.eq(DOCUMENT_ID)))
        .set(documents::s3_thumbnail.eq::<Option<String>>(Some("prefix/_thumb.png".into())))
        .execute(db)
        .await
        .expect("document thumbnail update should succeed");
}

fn assert_blocked<T>(result: Result<T, ApiError>) {
    let err = match result {
        Ok(_) => panic!("operation should be blocked"),
        Err(err) => err,
    };
    assert_eq!(err.status, StatusCode::CONFLICT);
    assert_eq!(
        err.message,
        "This file is stored but is waiting for virus scanning. It will be available after a clean scan."
    );
}

#[tokio::test]
async fn unsafe_file_blocks_download_thumbnail_page_text_and_ocr_metadata() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    seed_pending_file(&mut db).await;

    assert_blocked(
        get_document_file_download_metadata(&mut db, DOCUMENT_ID, DOCUMENT_FILE_ID).await,
    );
    assert_blocked(
        get_document_file_thumbnail_metadata(&mut db, DOCUMENT_ID, DOCUMENT_FILE_ID).await,
    );
    assert_blocked(
        get_document_file_page_image_key(&mut db, DOCUMENT_ID, DOCUMENT_FILE_ID, 1).await,
    );
    assert_blocked(list_document_file_pages(&mut db, DOCUMENT_ID, DOCUMENT_FILE_ID).await);
    assert_blocked(list_document_file_ocr_pages(&mut db, DOCUMENT_ID, DOCUMENT_FILE_ID).await);
    assert_blocked(get_document_thumbnail_metadata(&mut db, DOCUMENT_ID).await);
}
