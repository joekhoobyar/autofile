use std::collections::HashMap;

use autofile_api::domain::classifier_blocks::{
    ClassifierBlock, ClassifierPattern, ClassifierRules,
};
use diesel::dsl::now;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::bb8;

use autofile_api::schema::{
    classifier_blocks, document_file_pages, document_files, document_metadatas, document_types,
    document_types_metadata_types, documents, metadata_types, users,
};

pub async fn seed_classifier_document_scenario(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
) -> i64 {
    insert_user(db, 1, "tester", "tester@example.com").await;
    insert_document_type(db, 1, "invoice", "Invoice", 1).await;
    insert_metadata_type(db, 1, "category", "Category", 1).await;
    insert_metadata_type(db, 2, "invoice_number", "Invoice Number", 1).await;
    link_document_type_metadata(db, 1, 1).await;
    link_document_type_metadata(db, 1, 2).await;
    insert_document(db, 1, "Original Title", 1, 1).await;
    insert_document_metadata(db, 1, 1, "fallback", 1).await;
    insert_document_file(db, 1, 1, "invoice.pdf", 1).await;
    insert_document_file_page(db, 1, 1, "Invoice #123").await;

    insert_classifier_block(
        db,
        build_block(
            1,
            1,
            build_rules(
                true,
                vec![ClassifierPattern {
                    text: Some("Invoice".to_string()),
                    metadata: None,
                }],
                &[("category", "derived")],
            ),
        ),
    )
    .await;
    insert_classifier_block(
        db,
        build_block(
            2,
            2,
            build_rules(
                false,
                vec![ClassifierPattern {
                    text: None,
                    metadata: Some(HashMap::from([(
                        "category".to_string(),
                        "derived".to_string(),
                    )])),
                }],
                &[
                    ("_suggested_filename", "Classified Title"),
                    ("invoice_number", "123"),
                ],
            ),
        ),
    )
    .await;

    1
}

pub fn build_rules(
    continue_after_match: bool,
    match_patterns: Vec<ClassifierPattern>,
    match_actions: &[(&str, &str)],
) -> ClassifierRules {
    ClassifierRules {
        continue_after_match,
        match_patterns,
        match_actions: match_actions
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect(),
        child_rules: Vec::new(),
    }
}

pub fn build_block(id: i64, order: i32, rules: ClassifierRules) -> ClassifierBlock {
    use chrono::Utc;

    ClassifierBlock {
        id,
        name: format!("Block {id}"),
        description: None,
        enabled: true,
        order,
        rules: diesel_json::Json(rules),
        created_by: 1,
        created_at: Utc::now(),
        updated_by: 1,
        updated_at: Utc::now(),
    }
}

pub async fn insert_user(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    id: i64,
    username: &str,
    email: &str,
) {
    diesel::insert_into(users::table)
        .values((
            users::id.eq(id),
            users::username.eq(username),
            users::display_name.eq("Test User"),
            users::email.eq(email),
            users::password_hash.eq("hash"),
            users::password_changed_at.eq(now),
        ))
        .execute(db)
        .await
        .expect("user insert should succeed");
}

pub async fn insert_document_type(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    id: i64,
    slug: &str,
    name: &str,
    user_id: i64,
) {
    diesel::insert_into(document_types::table)
        .values((
            document_types::id.eq(id),
            document_types::slug.eq(slug),
            document_types::name.eq(name),
            document_types::description.eq::<Option<String>>(None),
            document_types::created_by.eq(user_id),
            document_types::updated_by.eq(user_id),
        ))
        .execute(db)
        .await
        .expect("document type insert should succeed");
}

pub async fn insert_metadata_type(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    id: i64,
    slug: &str,
    name: &str,
    user_id: i64,
) {
    diesel::insert_into(metadata_types::table)
        .values((
            metadata_types::id.eq(id),
            metadata_types::slug.eq(slug),
            metadata_types::name.eq(name),
            metadata_types::data_type.eq("string"),
            metadata_types::description.eq::<Option<String>>(None),
            metadata_types::options.eq::<Option<serde_json::Value>>(None),
            metadata_types::created_by.eq(user_id),
            metadata_types::updated_by.eq(user_id),
        ))
        .execute(db)
        .await
        .expect("metadata type insert should succeed");
}

pub async fn link_document_type_metadata(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_type_id: i64,
    metadata_type_id: i64,
) {
    diesel::insert_into(document_types_metadata_types::table)
        .values((
            document_types_metadata_types::document_type_id.eq(document_type_id),
            document_types_metadata_types::metadata_type_id.eq(metadata_type_id),
            document_types_metadata_types::required.eq(false),
            document_types_metadata_types::updated_at.eq(now),
        ))
        .execute(db)
        .await
        .expect("document type metadata link insert should succeed");
}

pub async fn insert_document(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    id: i64,
    title: &str,
    document_type_id: i64,
    user_id: i64,
) {
    diesel::insert_into(documents::table)
        .values((
            documents::id.eq(id),
            documents::title.eq(title),
            documents::document_type_id.eq(document_type_id),
            documents::created_by.eq(user_id),
            documents::updated_by.eq(user_id),
            documents::s3_thumbnail.eq::<Option<String>>(None),
        ))
        .execute(db)
        .await
        .expect("document insert should succeed");
}

pub async fn insert_document_metadata(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_id: i64,
    metadata_type_id: i64,
    value: &str,
    user_id: i64,
) {
    diesel::insert_into(document_metadatas::table)
        .values((
            document_metadatas::document_id.eq(document_id),
            document_metadatas::metadata_type_id.eq(metadata_type_id),
            document_metadatas::value.eq(value),
            document_metadatas::created_by.eq(user_id),
            document_metadatas::updated_by.eq(user_id),
        ))
        .execute(db)
        .await
        .expect("document metadata insert should succeed");
}

pub async fn insert_document_file(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    id: i64,
    document_id: i64,
    filename: &str,
    user_id: i64,
) {
    diesel::insert_into(document_files::table)
        .values((
            document_files::id.eq(id),
            document_files::document_id.eq(document_id),
            document_files::s3_prefix.eq("prefix"),
            document_files::filename.eq(filename),
            document_files::content_type.eq::<Option<String>>(Some("application/pdf".to_string())),
            document_files::size.eq(42_i64),
            document_files::created_by.eq(user_id),
            document_files::updated_by.eq(user_id),
            document_files::pages.eq(1_i32),
        ))
        .execute(db)
        .await
        .expect("document file insert should succeed");
}

pub async fn insert_document_file_page(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    document_file_id: i64,
    page_number: i32,
    text_content: &str,
) {
    diesel::insert_into(document_file_pages::table)
        .values((
            document_file_pages::document_file_id.eq(document_file_id),
            document_file_pages::page_number.eq(page_number),
            document_file_pages::text_content.eq::<Option<String>>(Some(text_content.to_string())),
        ))
        .execute(db)
        .await
        .expect("document file page insert should succeed");
}

pub async fn insert_classifier_block(
    db: &mut bb8::PooledConnection<'_, AsyncPgConnection>,
    block: ClassifierBlock,
) {
    diesel::insert_into(classifier_blocks::table)
        .values((
            classifier_blocks::id.eq(block.id),
            classifier_blocks::name.eq(block.name),
            classifier_blocks::description.eq(block.description),
            classifier_blocks::enabled.eq(block.enabled),
            classifier_blocks::order.eq(block.order),
            classifier_blocks::rules.eq(block.rules),
            classifier_blocks::created_by.eq(block.created_by),
            classifier_blocks::updated_by.eq(block.updated_by),
        ))
        .execute(db)
        .await
        .expect("classifier block insert should succeed");
}
