mod support;

use autofile_api::application::classifier_blocks::{
    compute_classification_actions, load_classifier_blocks, load_document_text,
    persist_computed_actions,
};
use autofile_api::application::documents::get_document_view;

use support::db::TestDatabase;
use support::fixtures::seed_classifier_document_scenario;

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
