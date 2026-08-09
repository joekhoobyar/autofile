mod support;

use autofile_api::application::document_index_documents::rebuild_document_index_inner;
use autofile_api::application::document_index_values::count_document_index_value_documents;
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

async fn list_index_value_rows(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_index_id: i64,
) -> Vec<(String, Option<i64>, bool)> {
    document_index_values::table
        .filter(document_index_values::document_index_id.eq(document_index_id))
        .order(document_index_values::value.asc())
        .select((
            document_index_values::value,
            document_index_values::parent_id,
            document_index_values::is_leaf,
        ))
        .load(db)
        .await
        .expect("document index value rows should load")
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
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document(&mut db, 2, "Beta", 100, 1).await;
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
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document(&mut db, 2, "Beta", 100, 1).await;
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

#[tokio::test]
async fn rebuild_document_index_skips_empty_child_branch_but_keeps_matching_sibling() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "", true, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "{{ doc.title }}", true, Some(1), 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(list_index_values(&mut db, 1).await, vec!["Alpha", "root"]);
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        vec![(1, "Alpha".to_string())]
    );
}

#[tokio::test]
async fn rebuild_document_index_skips_empty_grandchild_but_keeps_sibling_grandchild() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "section", false, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "", true, Some(2), 1).await;
    insert_document_index_template(&mut db, 4, 1, "{{ doc.title }}", true, Some(2), 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(
        list_index_values(&mut db, 1).await,
        vec!["Alpha", "root", "section"]
    );
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        vec![(1, "Alpha".to_string())]
    );
}

#[tokio::test]
async fn rebuild_document_index_does_not_persist_partial_path_when_branch_has_no_matching_leaf() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "section", false, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "", true, Some(2), 1).await;

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
async fn rebuild_document_index_keeps_independent_roots_isolated() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root-a", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "", true, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "root-b", false, None, 1).await;
    insert_document_index_template(&mut db, 4, 1, "{{ doc.title }}", true, Some(3), 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(list_index_values(&mut db, 1).await, vec!["Alpha", "root-b"]);
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        vec![(1, "Alpha".to_string())]
    );
}

#[tokio::test]
async fn rebuild_document_index_removes_stale_leaf_when_branch_becomes_empty() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "section", false, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "", true, Some(2), 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "root", None, false).await;
    insert_document_index_value(&mut db, 11, 1, 2, "section", Some(10), false).await;
    insert_document_index_value(&mut db, 12, 1, 3, "Old Leaf", Some(11), true).await;
    insert_document_index_document(&mut db, 12, 1).await;

    rebuild_document_index_inner(1, test_db.pool.clone())
        .await
        .expect("rebuild should succeed");

    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    assert_eq!(
        list_index_value_rows(&mut db, 1).await,
        Vec::<(String, Option<i64>, bool)>::new()
    );
    assert_eq!(
        list_index_documents(&mut db, 1).await,
        Vec::<(i64, String)>::new()
    );
}

#[tokio::test]
async fn document_index_value_counts_distinct_documents_across_descendant_leaves() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document(&mut db, 2, "Beta", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "first", true, Some(1), 1).await;
    insert_document_index_template(&mut db, 3, 1, "second", true, Some(1), 1).await;
    insert_document_index_template(&mut db, 4, 1, "empty", true, Some(1), 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "root", None, false).await;
    insert_document_index_value(&mut db, 11, 1, 2, "first", Some(10), true).await;
    insert_document_index_value(&mut db, 12, 1, 3, "second", Some(10), true).await;
    insert_document_index_value(&mut db, 13, 1, 4, "empty", Some(10), true).await;
    insert_document_index_document(&mut db, 11, 1).await;
    insert_document_index_document(&mut db, 12, 1).await;
    insert_document_index_document(&mut db, 12, 2).await;

    let counts = count_document_index_value_documents(&mut db, 1, &[10, 11, 12, 13])
        .await
        .expect("document counts should load");

    assert_eq!(counts.get(&10), Some(&2));
    assert_eq!(counts.get(&11), Some(&1));
    assert_eq!(counts.get(&12), Some(&2));
    assert_eq!(counts.get(&13), Some(&0));
}

#[tokio::test]
async fn document_index_value_counts_reflect_membership_removal() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_document_type(&mut db, 100, "invoice", "Invoice", 1).await;
    insert_document(&mut db, 1, "Alpha", 100, 1).await;
    insert_document(&mut db, 2, "Beta", 100, 1).await;
    insert_document_index(&mut db, 1, "main-index", "Main Index", 1).await;
    insert_document_index_template(&mut db, 1, 1, "root", false, None, 1).await;
    insert_document_index_template(&mut db, 2, 1, "leaf", true, Some(1), 1).await;
    insert_document_index_value(&mut db, 10, 1, 1, "root", None, false).await;
    insert_document_index_value(&mut db, 11, 1, 2, "leaf", Some(10), true).await;
    insert_document_index_document(&mut db, 11, 1).await;
    insert_document_index_document(&mut db, 11, 2).await;

    diesel::delete(
        document_index_documents::table
            .filter(document_index_documents::document_index_value_id.eq(11))
            .filter(document_index_documents::document_id.eq(1)),
    )
    .execute(&mut db)
    .await
    .expect("membership delete should succeed");

    let counts = count_document_index_value_documents(&mut db, 1, &[10, 11])
        .await
        .expect("document counts should load");
    assert_eq!(counts.get(&10), Some(&1));
    assert_eq!(counts.get(&11), Some(&1));

    diesel::delete(
        document_index_documents::table
            .filter(document_index_documents::document_index_value_id.eq(11)),
    )
    .execute(&mut db)
    .await
    .expect("membership delete should succeed");

    let counts = count_document_index_value_documents(&mut db, 1, &[10, 11])
        .await
        .expect("document counts should load");
    assert_eq!(counts.get(&10), Some(&0));
    assert_eq!(counts.get(&11), Some(&0));
}
