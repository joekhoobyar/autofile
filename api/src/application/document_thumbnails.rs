use std::sync::Arc;

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;

use crate::application::document_files::{
    DocumentFileContentType, convert_csv_to_pdf, convert_html_to_pdf, convert_image_to_png,
    convert_markdown_to_pdf, convert_office_document_to_pdf, convert_plaintext_to_pdf,
    convert_tsv_to_pdf,
    parse_document_file_content_type, stage_document_file_from_s3, upload_png_to_s3,
};
use crate::domain::document_files::DocumentFile;
use crate::schema::document_files;
use crate::schema::documents;
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
    let mut db = state.db_pool.get().await?;
    let document_file = document_files::table
        .find(document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await?;

    // Download the file from S3 into a temp file.
    let (temp_dir, temp_file) =
        stage_document_file_from_s3(&document_file, "autofile-pages", state.clone()).await?;

    let result = async {
        let thumb_path = temp_dir.join("_thumb.png");
        match parse_document_file_content_type(
            document_file.content_type.as_deref(),
            document_file.filename.as_str(),
        )? {
            DocumentFileContentType::PlainText => {
                generate_plaintext_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::Markdown => {
                generate_markdown_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::Csv => {
                generate_csv_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::Tsv => {
                generate_tsv_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::OfficeDocument => {
                generate_office_document_thumbnail(
                    &temp_file,
                    document_file.filename.as_str(),
                    page,
                    width,
                    &temp_dir,
                )
                .await?
            }
            DocumentFileContentType::Html => {
                generate_html_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::Pdf => {
                generate_pdf_thumbnail(&temp_file, page, width, &temp_dir).await?
            }
            DocumentFileContentType::Image => {
                generate_image_thumbnail(&temp_file, width, &thumb_path, state.clone()).await?
            }
        }

        let thumb_key = format!("{}/_thumb.png", document_file.s3_prefix);
        upload_png_to_s3(&thumb_path, &thumb_key, state.clone()).await?;

        let updated: usize =
            diesel::update(documents::table.filter(documents::id.eq(document_file.document_id)))
                .set((documents::s3_thumbnail.eq(thumb_key),))
                .execute(&mut db)
                .await?;
        if updated == 0 {
            tracing::warn!(
                "Document {} not found when updating thumbnail",
                document_file.document_id
            );
        }

        Ok(())
    }
    .await;

    if let Err(err) = tokio::fs::remove_dir_all(&temp_dir).await {
        tracing::warn!(error = %err, path = %temp_file, "failed to remove temp dir");
    }

    result
}

async fn generate_plaintext_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_plaintext_to_pdf(temp_file).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_markdown_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_markdown_to_pdf(temp_file).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_csv_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_csv_to_pdf(temp_file).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_tsv_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_tsv_to_pdf(temp_file).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_office_document_thumbnail(
    temp_file: &str,
    original_filename: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_office_document_to_pdf(temp_file, original_filename).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_html_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
    let pdf_file = convert_html_to_pdf(temp_file).await?;

    generate_pdf_thumbnail(pdf_file.as_str(), page, width, temp_dir).await?;
    Ok(())
}

async fn generate_pdf_thumbnail(
    temp_file: &str,
    page: u32,
    width: u32,
    temp_dir: &std::path::Path,
) -> JobResult<()> {
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
        .arg(temp_file)
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

    Ok(())
}

async fn generate_image_thumbnail(
    temp_file: &str,
    width: u32,
    output_path: &std::path::Path,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let normalized_path = output_path.with_file_name("_thumb-source.png");
    convert_image_to_png(temp_file.to_owned(), &normalized_path, state).await?;

    let output = Command::new("magick")
        .arg(&normalized_path)
        .arg("-thumbnail")
        .arg(format!("{}x", width))
        .arg(output_path)
        .output()
        .await?;

    if !output.status.success() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "magick thumbnail generation failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        )
        .into());
    }

    Ok(())
}
