use std::path::PathBuf;
use std::sync::Arc;

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;
use uuid::Uuid;

use crate::domain::document_files::DocumentFile;
use crate::schema::{document_file_pages, document_files};
use crate::shared::app_state::AppState;
use crate::shared::util::to_job_error;

/**
 * This job counts the pages in a document, and then extracts the text content for each page.
 */
pub async fn process_file_pages(
    document_file_id: i64,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    tracing::info!(document_file_id, "processing file pages");

    // Load the document file from the database.
    let mut db = state
        .db_pool
        .get()
        .await
        .map_err(to_job_error)?;
    let document_file = document_files::table
        .find(document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await
        .map_err(to_job_error)?;

    // Download the file from S3 into a temp file.
    let (temp_dir, temp_file) = stage_document_file_from_s3(&document_file, "autofile-pages", state.clone())
        .await?;

    let result = async {
        tracing::info!(document_file_id, "counting pages");
        // Count the pages in the document, then update the database
        let pages = count_pages(temp_file.clone(), state.clone())
            .await?;

        diesel::update(document_files::table.find(document_file_id))
            .set(document_files::pages.eq(pages as i32))
            .execute(&mut db)
            .await
            .map_err(to_job_error)?;

        // Extract the text for each page, and save it to the database.
        // call extract_page_text for each page, and save the text to the database
        // in a DocumentFilePage record.
        for page in 1..=pages {
            tracing::info!(document_file_id, page, "extracting text for page");
            let text = extract_page_text(temp_file.clone(), page, state.clone())
                .await?;
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
                .await
                .map_err(to_job_error)?;
        }

        Ok(())
    }
    .await;

    if let Err(err) = tokio::fs::remove_file(&temp_file).await {
        tracing::warn!(error = %err, path = %temp_file, "failed to remove temp file");
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

async fn extract_page_text(
    file: String,
    page: u32,
    _state: Data<Arc<AppState>>,
) -> Result<String, Error> {

    // 2) run `pdftotext -f {page} -l {page} input.pdf -` to extract text for the page
    let output = Command::new("pdftotext")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg(file)
        .arg("-")
        .output()
        .await
        .map_err(to_job_error)?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "pdftotext failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(to_job_error(error));
    }

    let text = String::from_utf8(output.stdout).map_err(to_job_error)?;
    Ok(text)
}

async fn count_pages(
    file: String,
    _state: Data<Arc<AppState>>,
) -> Result<u32, Error> {

    // 2) run `pdfinfo input.pdf` and parse the output to get the page count
    let output = Command::new("pdfinfo")
        .arg(file)
        .output()
        .await
        .map_err(to_job_error)?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "pdfinfo failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(to_job_error(error));
    }

    let output_str = String::from_utf8(output.stdout).map_err(to_job_error)?;
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

    Err(to_job_error(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Failed to parse page count from pdfinfo output",
    )))
}

pub async fn stage_document_file_from_s3(
    document_file: &DocumentFile,
    tempfile_prefix: &str,
    state: Data<Arc<AppState>>
) -> Result<(PathBuf, String), Error> {

    // Download the file from object storage, into a temp file.
    let s3_key = format!("{}/{}", document_file.s3_prefix, document_file.filename);
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(&s3_key)
        .send()
        .await
        .map_err(to_job_error)?;
    let file_bytes = object
        .body
        .collect()
        .await
        .map_err(to_job_error)?
        .into_bytes();
    let tmp_dir = std::env::temp_dir().join(format!("{}-{}", tempfile_prefix, Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(to_job_error)?;
    let tmp_file = tmp_dir.join("staged-file");
    tokio::fs::write(&tmp_file, file_bytes)
        .await
        .map_err(to_job_error)?;

    Ok((tmp_dir, tmp_file.to_string_lossy().to_string()))
}
