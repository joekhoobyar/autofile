mod support;

use autofile_api::application::document_files::{cleanup_extra_page_rows, stale_page_image_keys};
use autofile_api::schema::{document_file_ocr_pages, document_file_pages};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use support::db::TestDatabase;
use support::fixtures::{
    insert_document, insert_document_file, insert_document_file_page, insert_document_type,
    insert_user,
};

async fn list_text_pages(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_file_id: i64,
) -> Vec<i32> {
    document_file_pages::table
        .filter(document_file_pages::document_file_id.eq(document_file_id))
        .order(document_file_pages::page_number.asc())
        .select(document_file_pages::page_number)
        .load(db)
        .await
        .expect("text pages should load")
}

async fn list_ocr_pages(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_file_id: i64,
) -> Vec<i32> {
    document_file_ocr_pages::table
        .filter(document_file_ocr_pages::document_file_id.eq(document_file_id))
        .order(document_file_ocr_pages::page_number.asc())
        .select(document_file_ocr_pages::page_number)
        .load(db)
        .await
        .expect("ocr pages should load")
}

#[tokio::test]
async fn cleanup_extra_page_rows_removes_rows_above_target_page_count() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 1, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Invoice", 1, 1).await;
    insert_document_file(&mut db, 1, 1, "invoice.pdf", 1).await;

    insert_document_file_page(&mut db, 1, 1, "page one").await;
    insert_document_file_page(&mut db, 1, 2, "page two").await;
    insert_document_file_page(&mut db, 1, 3, "page three").await;

    diesel::insert_into(document_file_ocr_pages::table)
        .values((
            document_file_ocr_pages::document_file_id.eq(1_i64),
            document_file_ocr_pages::page_number.eq(1_i32),
            document_file_ocr_pages::ocr_content.eq::<Option<String>>(Some("ocr one".into())),
        ))
        .execute(&mut db)
        .await
        .expect("ocr page 1 insert should succeed");
    diesel::insert_into(document_file_ocr_pages::table)
        .values((
            document_file_ocr_pages::document_file_id.eq(1_i64),
            document_file_ocr_pages::page_number.eq(2_i32),
            document_file_ocr_pages::ocr_content.eq::<Option<String>>(Some("ocr two".into())),
        ))
        .execute(&mut db)
        .await
        .expect("ocr page 2 insert should succeed");
    diesel::insert_into(document_file_ocr_pages::table)
        .values((
            document_file_ocr_pages::document_file_id.eq(1_i64),
            document_file_ocr_pages::page_number.eq(3_i32),
            document_file_ocr_pages::ocr_content.eq::<Option<String>>(Some("ocr three".into())),
        ))
        .execute(&mut db)
        .await
        .expect("ocr page 3 insert should succeed");

    cleanup_extra_page_rows(&mut db, 1, 2)
        .await
        .expect("cleanup should succeed");

    assert_eq!(list_text_pages(&mut db, 1).await, vec![1, 2]);
    assert_eq!(list_ocr_pages(&mut db, 1).await, vec![1, 2]);
}

#[test]
fn stale_page_image_keys_returns_only_stale_pages() {
    assert_eq!(
        stale_page_image_keys("prefix", 3, 2),
        vec!["prefix/pages/3.png".to_string()]
    );
}

#[test]
fn stale_page_image_keys_returns_empty_when_not_shrinking() {
    assert!(stale_page_image_keys("prefix", 2, 2).is_empty());
    assert!(stale_page_image_keys("prefix", 2, 3).is_empty());
}
