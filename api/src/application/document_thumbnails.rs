use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;

use crate::application::document_files::stage_document_file_from_s3;
use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::upload_to_s3;
use crate::schema::documents;
use crate::schema::document_files;
use crate::shared::app_state::AppState;
use crate::shared::util::JobResult;

/**
 * This job generates a thumbnail for a given document file and page number,
 * and updates the document record with the thumbnail's S3 key.
 */
pub async fn generate_thumbnail(
    document_file_id: i64,
    page: u32,
    width: u32,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    generate_thumbnail_inner(document_file_id, page, width, state)
        .await
        .map_err(Into::into)
}

async fn generate_thumbnail_inner(
    document_file_id: i64,
    page: u32,
    width: u32,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    tracing::info!(document_file_id, page, width, "generating thumbnail");

    // Load the document file from the database to get the S3 location.
    let mut db = state
        .db_pool
        .get()
        .await?;
    let document_file = document_files::table
        .find(document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await?;

    // Download the file from S3 into a temp file.
    let (temp_dir, temp_file) = stage_document_file_from_s3(&document_file, "autofile-pages", state.clone())
        .await?;

    // generate thumbnail
    let output_prefix = temp_dir.join("_thumb");
    let status = Command::new("pdftocairo")
        .arg("-png")
        .arg("-singlefile")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg("-scale-to-x")
        .arg(width.to_string())
        .arg("-scale-to-y")
        .arg("-1")
        .arg(&temp_file)
        .arg(&output_prefix)
        .status()
        .await?;
    if !status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("pdftocairo failed with status {status}"),
        );
        return Err(error.into());
    }

    // upload thumbnail
    let thumb_path = temp_dir.join("_thumb.png");
    let thumb_key = format!("{}/_thumb.png", document_file.s3_prefix);
    let body = ByteStream::from_path(&thumb_path)
        .await?;
    upload_to_s3(
        &state.s3_client,
        state.s3_bucket.as_str(),
        &thumb_key,
        body,
        Some("image/png"),
    )
    .await?;

    // Store the thumbnail in the documents table
    let updated: usize =
        diesel::update(documents::table.filter(documents::id.eq(document_file.document_id)))
            .set((
                documents::s3_thumbnail.eq(thumb_key),
            ))
            .execute(&mut db)
            .await?;
    if updated == 0 {
        tracing::warn!("Document {} not found when updating thumbnail", document_file.document_id);
    }

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    Ok(())
}
