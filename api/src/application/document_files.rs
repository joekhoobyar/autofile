use std::path::PathBuf;
use std::sync::Arc;

use apalis::prelude::*;
use aws_sdk_s3::primitives::ByteStream;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;
use uuid::Uuid;

use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::upload_to_s3;
use crate::schema::{document_file_ocr_pages, document_file_pages, document_files};
use crate::shared::app_state::AppState;
use crate::shared::util::JobResult;

/**
 * This job counts the pages in a document, and then extracts the text content for each page.
 */
pub async fn process_file_pages(
    document_file_id: i64,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    process_file_pages_inner(document_file_id, state)
        .await
        .map_err(Into::into)
}

/**
 * Internal function to process the pages of a document file by counting the pages
 * and extracting the text content for each page.
 */
async fn process_file_pages_inner(
    document_file_id: i64,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    tracing::info!(document_file_id, "processing file pages");

    // Load the document file from the database.
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
        tracing::info!(document_file_id, "counting pages");
        // Count the pages in the document, then update the database
        let pages = count_pages(temp_file.clone(), state.clone()).await?;

        diesel::update(document_files::table.find(document_file_id))
            .set(document_files::pages.eq(pages as i32))
            .execute(&mut db)
            .await?;

        // Extract the text for each page, and save it to the database.
        // call extract_page_text for each page, and save the text to the database
        // in a DocumentFilePage record.
        for page in 1..=pages {
            tracing::info!(document_file_id, page, "extracting text for page");
            let text = extract_page_text(temp_file.clone(), page, state.clone()).await?;
            diesel::insert_into(document_file_pages::table)
                .values(&NewDocumentFilePage {
                    document_file_id,
                    page_number: page as i32,
                    text_content: Some(text),
                })
                .on_conflict((
                    document_file_pages::document_file_id,
                    document_file_pages::page_number,
                ))
                .do_update()
                .set(
                    document_file_pages::text_content
                        .eq(diesel::upsert::excluded(document_file_pages::text_content)),
                )
                .execute(&mut db)
                .await?;

            tracing::info!(document_file_id, page, "extracting image for page");
            let image_path = extract_page_image(
                temp_file.clone(),
                page,
                &temp_dir,
                &document_file.s3_prefix,
                state.clone(),
            )
            .await?;

            tracing::info!(document_file_id, page, "extracting OCR text for page");
            let ocr_text = extract_page_ocr(image_path, state.clone()).await?;
            diesel::insert_into(document_file_ocr_pages::table)
                .values(&NewDocumentFileOcrPage {
                    document_file_id,
                    page_number: page as i32,
                    ocr_content: Some(ocr_text),
                })
                .on_conflict((
                    document_file_ocr_pages::document_file_id,
                    document_file_ocr_pages::page_number,
                ))
                .do_update()
                .set(
                    document_file_ocr_pages::ocr_content.eq(diesel::upsert::excluded(
                        document_file_ocr_pages::ocr_content,
                    )),
                )
                .execute(&mut db)
                .await?;
        }

        Ok(())
    }
    .await;

    if let Err(err) = tokio::fs::remove_dir_all(&temp_dir).await {
        tracing::warn!(error = %err, path = %temp_file, "failed to remove temp dir");
    }

    result
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_file_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentFilePage {
    document_file_id: i64,
    page_number: i32,
    text_content: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_file_ocr_pages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentFileOcrPage {
    document_file_id: i64,
    page_number: i32,
    ocr_content: Option<String>,
}

/**
 * Internal function to extract the text content of a specific page in a PDF document
 * by running `pdftotext` on the file and capturing the output.
 */
async fn extract_page_text(
    file: String,
    page: u32,
    _state: Data<Arc<AppState>>,
) -> JobResult<String> {
    let output = Command::new("pdftotext")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(file)
        .arg("-")
        .output()
        .await?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "pdftotext failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(error.into());
    }

    let text = String::from_utf8(output.stdout)?;
    Ok(text)
}

/**
 * Internal function to extract the image content of a specific page in a PDF document
 * by running `pdftocairo` on the file and capturing the output.
 */
async fn extract_page_image(
    file: String,
    page: u32,
    temp_dir: &PathBuf,
    s3_prefix: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<PathBuf> {
    let output_prefix = temp_dir.join(format!("page-{}", page));
    let status = Command::new("pdftocairo")
        .arg("-png")
        .arg("-singlefile")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg("-scale-to-x")
        .arg("2400")
        .arg("-scale-to-y")
        .arg("-1")
        .arg(file)
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

    let output_path = temp_dir.join(format!("page-{}.png", page));
    let s3_key = format!("{}/pages/{}.png", s3_prefix, page);
    let body = ByteStream::from_path(&output_path).await?;
    upload_to_s3(
        &state.s3_client,
        state.s3_bucket.as_str(),
        &s3_key,
        body,
        Some("image/png"),
    )
    .await?;

    Ok(output_path)
}

/**
 * Internal function to extract OCR text from a page image
 * by running `tesseract` on the file and capturing the output.
 */
async fn extract_page_ocr(image_path: PathBuf, _state: Data<Arc<AppState>>) -> JobResult<String> {
    let output = Command::new("tesseract")
        .arg(&image_path)
        .arg("stdout")
        .output()
        .await?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "tesseract failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(error.into());
    }

    let text = String::from_utf8(output.stdout)?;
    Ok(text)
}

/**
 * Internal function to count the number of pages in a PDF document
 * by running `pdfinfo` on the file and parsing the output.
 */
async fn count_pages(file: String, _state: Data<Arc<AppState>>) -> JobResult<u32> {
    // 2) run `pdfinfo input.pdf` and parse the output to get the page count
    let output = Command::new("pdfinfo").arg(file).output().await?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "pdfinfo failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(error.into());
    }

    let output_str = String::from_utf8(output.stdout)?;
    for line in output_str.lines() {
        if line.starts_with("Pages:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2 {
                if let Ok(page_count) = parts[1].parse::<u32>() {
                    return Ok(page_count);
                }
            }
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to parse page count from pdfinfo output",
    )
    .into())
}

/**
 * Downloads a document file from S3 into a temporary file.
 * Returns the path to the temp directory as a PathBuf, and the path to the temp file as a String.
 */
pub async fn stage_document_file_from_s3(
    document_file: &DocumentFile,
    tempfile_prefix: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<(PathBuf, String)> {
    // Download the file from object storage, into a temp file.
    let s3_key = format!("{}/{}", document_file.s3_prefix, document_file.filename);
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(&s3_key)
        .send()
        .await?;
    let file_bytes = object.body.collect().await?.into_bytes();
    let tmp_dir = std::env::temp_dir().join(format!("{}-{}", tempfile_prefix, Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp_dir).await?;
    let tmp_file = tmp_dir.join("staged-file");
    tokio::fs::write(&tmp_file, file_bytes).await?;

    Ok((tmp_dir, tmp_file.to_string_lossy().to_string()))
}
