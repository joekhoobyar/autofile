use std::sync::OnceLock;

use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};

pub mod schema;
pub mod api {
    pub mod auth;
    pub mod cabinet_documents;
    pub mod cabinets;
    pub mod classifier_blocks;
    pub mod document_file_pages;
    pub mod document_files;
    pub mod document_index_templates;
    pub mod document_index_values;
    pub mod document_indexes;
    pub mod document_metadatas;
    pub mod document_types;
    pub mod document_types_metadata_types;
    pub mod documents;
    pub mod metadata_types;
    pub mod tag_documents;
    pub mod tags;
    pub mod users;
}
pub mod application {
    pub mod classifier_blocks;
    pub mod document_files;
    pub mod document_index_documents;
    pub mod document_index_values;
    pub mod document_metadatas;
    pub mod document_thumbnails;
    pub mod documents;
    pub mod jobs;
    pub mod users;
}
pub mod domain {
    pub mod cabinet_documents;
    pub mod cabinets;
    pub mod classifier_blocks;
    pub mod document_files;
    pub mod document_indexes;
    pub mod document_metadatas;
    pub mod document_types;
    pub mod document_types_metadata_types;
    pub mod documents;
    pub mod metadata_types;
    pub mod tag_documents;
    pub mod tags;
    pub mod users;
}
pub mod infrastructure {
    pub mod s3;
}
pub mod shared {
    pub mod app_state;
    pub mod auth;
    pub mod extractors;
    pub mod s3;
    pub mod util;
}

pub use shared::app_state::AppState;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub async fn run_migrations(database_url: &str) {
    let database_url = database_url.to_string();

    tokio::task::spawn_blocking(move || {
        let mut conn = <diesel::PgConnection as diesel::Connection>::establish(&database_url)
            .expect("Failed to establish connection for migrations");

        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        tracing::info!("Migrations completed successfully");
    })
    .await
    .expect("Migration task panicked");
}

static IS_PRODUCTION: OnceLock<bool> = OnceLock::new();

pub fn is_production() -> bool {
    *IS_PRODUCTION.get_or_init(|| {
        std::env::var("APP_MODE")
            .map(|v| v.eq_ignore_ascii_case("production"))
            .unwrap_or(false)
    })
}
