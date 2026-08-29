mod support;

use autofile_api::application::classifier_blocks::{
    UpdateClassifierBlockInput, compute_classification_actions, create_classifier_block,
    delete_classifier_block, load_classifier_blocks, load_document_text, persist_computed_actions,
    reorder_classifier_block, update_classifier_block,
};
use autofile_api::application::documents::get_document_view;
use autofile_api::domain::classifier_blocks::ClassifierPattern;
use autofile_api::schema::classifier_blocks;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use support::db::TestDatabase;
use support::fixtures::{
    build_rules, insert_user, seed_classifier_blocks, seed_classifier_document_scenario,
};

async fn list_block_orders(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
) -> Vec<(i64, i32)> {
    classifier_blocks::table
        .order(classifier_blocks::order.asc())
        .select((classifier_blocks::id, classifier_blocks::order))
        .load(db)
        .await
        .expect("classifier blocks should load")
}

async fn list_block_names_and_orders(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
) -> Vec<(String, i32)> {
    classifier_blocks::table
        .order(classifier_blocks::order.asc())
        .select((classifier_blocks::name, classifier_blocks::order))
        .load(db)
        .await
        .expect("classifier blocks should load")
}

#[tokio::test]
async fn persists_computed_actions_without_enqueue_side_effects() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    let document_id = seed_classifier_document_scenario(&mut db).await;

    let document_view = get_document_view(&mut db, document_id)
        .await
        .expect("document view should load");
    let document_text = load_document_text(&mut db, document_id)
        .await
        .expect("document text should load");
    let blocks = load_classifier_blocks(&mut db)
        .await
        .expect("classifier blocks should load");
    let computed_actions =
        compute_classification_actions(document_id, &document_view, &document_text, &blocks)
            .expect("classification should succeed");

    persist_computed_actions(&mut db, document_id, 1, computed_actions)
        .await
        .expect("classification persistence should succeed");

    let refreshed = get_document_view(&mut db, document_id)
        .await
        .expect("updated document view should load");

    assert_eq!(refreshed.title, "Classified Title");
    assert_eq!(
        refreshed.metadata.get("category"),
        Some(&"derived".to_string())
    );
    assert_eq!(
        refreshed.metadata.get("invoice_number"),
        Some(&"123".to_string())
    );
}

#[tokio::test]
async fn reorder_moves_block_up_and_shifts_intervening_rows() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "reorderer", "reorderer@example.com").await;
    seed_classifier_blocks(&mut db, 4).await;

    let reordered = reorder_classifier_block(&mut db, 42, 3, 1)
        .await
        .expect("reorder should succeed");

    assert_eq!(reordered.id, 3);
    assert_eq!(reordered.order, 1);
    assert_eq!(reordered.updated_by, 42);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(3, 1), (1, 2), (2, 3), (4, 4)]
    );
}

#[tokio::test]
async fn reorder_moves_block_down_and_shifts_intervening_rows() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "reorderer", "reorderer@example.com").await;
    seed_classifier_blocks(&mut db, 4).await;

    let reordered = reorder_classifier_block(&mut db, 42, 1, 4)
        .await
        .expect("reorder should succeed");

    assert_eq!(reordered.id, 1);
    assert_eq!(reordered.order, 4);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(2, 1), (3, 2), (4, 3), (1, 4)]
    );
}

#[tokio::test]
async fn reorder_same_position_is_noop() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "reorderer", "reorderer@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let reordered = reorder_classifier_block(&mut db, 42, 2, 2)
        .await
        .expect("reorder should succeed");

    assert_eq!(reordered.id, 2);
    assert_eq!(reordered.order, 2);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn reorder_rejects_out_of_bounds_order() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "reorderer", "reorderer@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let err = reorder_classifier_block(&mut db, 42, 2, 0)
        .await
        .expect_err("reorder should fail");

    assert_eq!(err.status, 400);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn reorder_returns_not_found_for_missing_block() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "reorderer", "reorderer@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let err = reorder_classifier_block(&mut db, 42, 999, 1)
        .await
        .expect_err("reorder should fail");

    assert_eq!(err.status, 404);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn create_classifier_block_appends_to_end_of_ordered_list() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let created = create_classifier_block(
        &mut db,
        1,
        "New Block".to_string(),
        Some("Added in test".to_string()),
        true,
        build_rules(
            false,
            vec![ClassifierPattern {
                text: Some("Invoice".to_string()),
                metadata: None,
            }],
            &[],
        ),
    )
    .await
    .expect("create should succeed");

    assert_eq!(created.name, "New Block");
    assert_eq!(created.order, 4);
    assert_eq!(created.created_by, 1);
    assert_eq!(
        list_block_names_and_orders(&mut db).await,
        vec![
            ("Block 1".to_string(), 1),
            ("Block 2".to_string(), 2),
            ("Block 3".to_string(), 3),
            ("New Block".to_string(), 4),
        ]
    );
}

#[tokio::test]
async fn delete_classifier_block_closes_order_gap() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    seed_classifier_blocks(&mut db, 4).await;

    delete_classifier_block(&mut db, 2)
        .await
        .expect("delete should succeed");

    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (3, 2), (4, 3)]
    );
}

#[tokio::test]
async fn delete_classifier_block_returns_not_found_for_missing_block() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let err = delete_classifier_block(&mut db, 999)
        .await
        .expect_err("delete should fail");

    assert_eq!(err.status, 404);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn update_classifier_block_updates_selected_fields_without_changing_order() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "updater", "updater@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let updated = update_classifier_block(
        &mut db,
        42,
        2,
        UpdateClassifierBlockInput {
            name: Some("Updated Block".to_string()),
            description: Some("Updated description".to_string()),
            enabled: Some(false),
            rules: Some(build_rules(
                false,
                vec![ClassifierPattern {
                    text: Some("Updated".to_string()),
                    metadata: None,
                }],
                &[("status", "updated")],
            )),
        },
    )
    .await
    .expect("update should succeed");

    assert_eq!(updated.id, 2);
    assert_eq!(updated.name, "Updated Block");
    assert_eq!(updated.description.as_deref(), Some("Updated description"));
    assert!(!updated.enabled);
    assert_eq!(updated.order, 2);
    assert_eq!(updated.updated_by, 42);
    assert_eq!(
        updated.rules.0.match_actions.get("status"),
        Some(&"updated".to_string())
    );
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn update_classifier_block_returns_not_found_for_missing_block() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "updater", "updater@example.com").await;
    seed_classifier_blocks(&mut db, 3).await;

    let err = update_classifier_block(
        &mut db,
        42,
        999,
        UpdateClassifierBlockInput {
            name: Some("Missing".to_string()),
            description: None,
            enabled: None,
            rules: None,
        },
    )
    .await
    .expect_err("update should fail");

    assert_eq!(err.status, 404);
    assert_eq!(
        list_block_orders(&mut db).await,
        vec![(1, 1), (2, 2), (3, 3)]
    );
}

#[tokio::test]
async fn create_classifier_block_rejects_rules_without_match_patterns() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;

    let err = create_classifier_block(
        &mut db,
        1,
        "Invalid Block".to_string(),
        None,
        true,
        build_rules(false, vec![], &[]),
    )
    .await
    .expect_err("create should fail");

    assert_eq!(err.status, 422);
    assert!(
        err.message
            .contains("At least one match pattern is required")
    );
    assert!(list_block_orders(&mut db).await.is_empty());
}

#[tokio::test]
async fn update_classifier_block_rejects_empty_patterns_but_allows_other_updates() {
    let test_db = TestDatabase::new().await;
    let mut db = test_db
        .pool
        .get()
        .await
        .expect("db connection should succeed");
    insert_user(&mut db, 1, "tester", "tester@example.com").await;
    insert_user(&mut db, 42, "updater", "updater@example.com").await;
    seed_classifier_blocks(&mut db, 1).await;

    let invalid = update_classifier_block(
        &mut db,
        42,
        1,
        UpdateClassifierBlockInput {
            name: None,
            description: None,
            enabled: None,
            rules: Some(build_rules(
                false,
                vec![ClassifierPattern {
                    text: Some("".to_string()),
                    metadata: None,
                }],
                &[],
            )),
        },
    )
    .await
    .expect_err("rules update should fail");
    assert_eq!(invalid.status, 422);

    let updated = update_classifier_block(
        &mut db,
        42,
        1,
        UpdateClassifierBlockInput {
            name: Some("Legacy Block".to_string()),
            description: None,
            enabled: None,
            rules: None,
        },
    )
    .await
    .expect("non-rules update should preserve legacy rules");

    assert_eq!(updated.name, "Legacy Block");
    assert!(updated.rules.0.match_patterns.is_empty());
}
