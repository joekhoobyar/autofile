mod support;

use autofile_api::application::document_index_documents::rebuild_document_index_inner;
use autofile_api::schema::{document_index_documents, document_index_values};
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use support::db::TestDatabase;
use support::fixtures::{
    insert_document, insert_document_index, insert_document_index_document,
    insert_document_index_template, insert_document_index_value, insert_document_type, insert_user,
};

async fn list_index_values(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_index_id: i64,
) -> Vec<String> {
    document_index_values::table
        .filter(document_index_values::document_index_id.eq(document_index_id))
        .order(document_index_values::value.asc())
        .select(document_index_values::value)
        .load(db)
        .await
        .expect("document index values should load")
}

async fn list_index_documents(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_index_id: i64,
) -> Vec<(i64, String)> {
    document_index_documents::table
        .inner_join(
            document_index_values::table
                .on(document_index_documents::document_index_value_id.eq(document_index_values::id)),
        )
        .filter(document_index_values::document_index_id.eq(document_index_id))
        .order((document_index_documents::document_id.asc(), document_index_values::value.asc()))
        .select((document_index_documents::document_id, document_index_values::value))
        .load(db)
        .await
        .expect("document index documents should load")
}

#[tokio::test]
async fn rebuild_document_index_clears_stale_rows_and_rebuilds_from_documents() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 1, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 1, 1).await;
    insert_document(&mut db, 2, "Beta", 1, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "{{ doc.title }}", true, None, 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "STALE", None, true).await;
    insert_document_index_document(&mut db, 10, 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(list_index_values(&mut db, 1).await, vec!["Alpha", "Beta"]);
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        vec![(1, "Alpha".to_string()), (2, "Beta".to_string())]
    );
}

#[tokio::test]
async fn rebuild_document_index_only_affects_target_index() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 1, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 1, 1).await;
    insert_document(&mut db, 2, "Beta", 1, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index(&mut db, 2, "other-index", "Other Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "{{ doc.title }}", true, None, 1).await;
    insert_document_index_template(&mut db, 2, 2, "{{ doc.title }}", true, None, 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "STALE", None, true).await;
    insert_document_index_document(&mut db, 10, 1).await;
    insert_document_index_value(&mut db, 20, 2, 2, "Preserved", None, true).await;
    insert_document_index_document(&mut db, 20, 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(list_index_values(&mut db, 1).await, vec!["Alpha", "Beta"]);
    assert_eq!(list_index_values(&mut db, 2).await, vec!["Preserved"]);
    assert_eq!(
        list_index_documents(&mut db, 2).await,
        vec![(1, "Preserved".to_string())]
    );
}

#[tokio::test]
async fn rebuild_document_index_with_no_documents_only_clears_existing_rows() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "{{ doc.title }}", true, None, 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "STALE", None, true).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(list_index_values(&mut db, 1).await, Vec::<String>::new());
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        Vec::<(i64, String)>::new()
    );
}

#[tokio::test]
async fn rebuild_nonexistent_document_index_is_successful_noop() {
    let test_db = TestDatabase::new().await;

    rebuild_document_index_inner(999, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");
}
