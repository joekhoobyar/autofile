use std::sync::Arc;

use serde::{Deserialize, Serialize};

use apalis::prelude::*;

use crate::application::document_files::process_file_pages;
use crate::application::document_index_documents::update_document_index_document;
use crate::application::document_thumbnails::generate_thumbnail;
use crate::shared::app_state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FastJob {
    GenerateThumbnail {
        document_file_id: i64,
        page: u32,
        width: u32,
    },
    ProcessFilePages {
        document_file_id: i64,
    },
    UpdateDocumentIndexDocument {
        document_index_id: i64,
        document_id: i64,
    },
}

/**
 * Handles a fast job by matching on the job type and calling the appropriate function to process it.
 *
 * The function takes a `FastJob` and an `Arc<AppState>` as parameters and returns a `Result<(), Error>`.
 */
pub async fn handle_fast_job(job: FastJob, state: Data<Arc<AppState>>) -> Result<(), Error> {
    match job {
        FastJob::GenerateThumbnail {
            document_file_id,
            page,
            width,
        } => generate_thumbnail(document_file_id, page, width, state).await,
        FastJob::UpdateDocumentIndexDocument {
            document_index_id,
            document_id,
        } => update_document_index_document(document_index_id, document_id, state).await,
        FastJob::ProcessFilePages { document_file_id } => {
            process_file_pages(document_file_id, state).await
        }
    }
}
