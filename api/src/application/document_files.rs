use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use aws_sdk_s3::primitives::ByteStream;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;
use uuid::Uuid;

use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::delete_from_s3;
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

    drop(db);

    let result = async {
        match parse_document_file_content_type(
            document_file.content_type.as_deref(),
            document_file.filename.as_str(),
        )? {
            DocumentFileContentType::PlainText => {
                process_file_pages_plaintext(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::Html => {
                process_file_pages_html(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::Markdown => {
                process_file_pages_markdown(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::Pdf => {
                process_file_pages_pdf(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::Image => {
                process_file_pages_image(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
        }
    }
    .await;

    if let Err(err) = tokio::fs::remove_dir_all(&temp_dir).await {
        tracing::warn!(error = %err, path = %temp_file, "failed to remove temp dir");
    }

    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DocumentFileContentType {
    Pdf,
    Image,
    Markdown,
    PlainText,
    Html,
}

pub(crate) fn parse_document_file_content_type(
    content_type: Option<&str>,
    filename: &str,
) -> JobResult<DocumentFileContentType> {
    let content_type = content_type.ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Document file is missing content_type",
        )
    })?;

    if content_type == "application/pdf" {
        return Ok(DocumentFileContentType::Pdf);
    }

    if content_type.starts_with("image/") {
        return Ok(DocumentFileContentType::Image);
    }

    if content_type == "text/markdown"
        || content_type == "text/x-markdown"
        || (content_type == "text/plain" && filename.ends_with(".md"))
    {
        return Ok(DocumentFileContentType::Markdown);
    }

    if content_type == "text/html"
        || content_type == "application/xhtml+xml"
        || (content_type == "text/plain"
            && (filename.ends_with(".html") || filename.ends_with(".htm")))
    {
        return Ok(DocumentFileContentType::Html);
    }

    if content_type == "text/plain" {
        return Ok(DocumentFileContentType::PlainText);
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        format!("Unsupported content_type for page processing: {content_type}"),
    )
    .into())
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
 * Internal function to extract the images and text from a plain text document 
 * by running `pandoc` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_plaintext(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file = convert_plaintext_to_pdf(temp_file).await?;

    process_file_pages_pdf(
        document_file_id,
        document_file,
        temp_dir,
        pdf_file.as_str(),
        state,
    )
    .await?;

    Ok(())
}

/**
 * Internal function to convert a text file to PDF by running `pandoc`.
 */
pub(crate) async fn convert_plaintext_to_pdf(text_file: &str) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", text_file);

    let status = Command::new("pandoc")
        .arg("-f")
        .arg("markdown")
        .arg(text_file)
        .arg("-o")
        .arg(pdf_file.as_str())
        .arg("--pdf-engine=xelatex")
        .status()
        .await?;
    if !status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("pandoc failed with status {status}"),
        );
        return Err(error.into());
    }

    Ok(pdf_file)
}

/**
 * Internal function to extract the images and text from a markdown document
 * by running `pandoc` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_markdown(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file = convert_markdown_to_pdf(temp_file).await?;

    process_file_pages_pdf(
        document_file_id,
        document_file,
        temp_dir,
        pdf_file.as_str(),
        state,
    )
    .await?;

    Ok(())
}

/**
 * Internal function to extract the images and text from an HTML document
 * by running `weasyprint` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_html(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file = convert_html_to_pdf(temp_file).await?;

    process_file_pages_pdf(
        document_file_id,
        document_file,
        temp_dir,
        pdf_file.as_str(),
        state,
    )
    .await?;

    Ok(())
}

/**
 * Internal function to convert an HTML file to PDF by running `weasyprint`.
 */
pub(crate) async fn convert_html_to_pdf(html_file: &str) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", html_file);

    let status = Command::new("weasyprint")
        .arg(html_file)
        .arg(pdf_file.as_str())
        .status()
        .await?;
    if !status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("weasyprint failed with status {status}"),
        );
        return Err(error.into());
    }

    Ok(pdf_file)
}

/**
 * Internal function to convert a markdown file to PDF by running `pandoc`.
 */
pub(crate) async fn convert_markdown_to_pdf(markdown_file: &str) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", markdown_file);

    let status = Command::new("pandoc")
        .arg("-f")
        .arg("markdown")
        .arg(markdown_file)
        .arg("-o")
        .arg(pdf_file.as_str())
        .arg("--pdf-engine=xelatex")
        .status()
        .await?;
    if !status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("pandoc failed with status {status}"),
        );
        return Err(error.into());
    }

    Ok(pdf_file)
}

/**
 * Internal function to extract the images and text from a PDF document
 * by running `pdftotext` on the file and capturing the output.
 */
async fn process_file_pages_pdf(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let mut db = state.db_pool.get().await?;

    let prev_pages = document_file.pages.max(0) as u32;

    tracing::info!(document_file_id, "counting pages");
    let pages = count_pages(temp_file.to_owned(), state.clone()).await?;

    diesel::update(document_files::table.find(document_file_id))
        .set(document_files::pages.eq(pages as i32))
        .execute(&mut db)
        .await?;

    for page in 1..=pages {
        tracing::info!(document_file_id, page, "extracting text for page");
        let text = extract_pdf_page_text(temp_file.to_owned(), page, state.clone()).await?;
        upsert_document_file_page(&mut db, document_file_id, page as i32, Some(text)).await?;

        tracing::info!(document_file_id, page, "extracting image for page");
        let image_path = extract_pdf_page_image(
            temp_file.to_owned(),
            page,
            temp_dir,
            &document_file.s3_prefix,
            state.clone(),
        )
        .await?;

        tracing::info!(document_file_id, page, "extracting OCR text for page");
        let ocr_text = extract_page_ocr(image_path, state.clone()).await?;
        upsert_document_file_ocr_page(&mut db, document_file_id, page as i32, Some(ocr_text))
            .await?;
    }

    cleanup_extra_pages(
        &mut db,
        document_file_id,
        &document_file.s3_prefix,
        pages,
        prev_pages,
        state.clone(),
    )
    .await?;

    Ok(())
}

async fn process_file_pages_image(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let mut db = state.db_pool.get().await?;

    let prev_pages = document_file.pages.max(0) as u32;

    diesel::update(document_files::table.find(document_file_id))
        .set(document_files::pages.eq(1))
        .execute(&mut db)
        .await?;

    upsert_document_file_page(&mut db, document_file_id, 1, None).await?;

    let image_path = temp_dir.join("page-1.png");
    convert_image_to_png(temp_file.to_owned(), &image_path, state.clone()).await?;
    upload_png_to_s3(
        &image_path,
        &format!("{}/pages/1.png", document_file.s3_prefix),
        state.clone(),
    )
    .await?;

    tracing::info!(
        document_file_id,
        page = 1,
        "extracting OCR text for image page"
    );
    let ocr_text = extract_page_ocr(image_path, state.clone()).await?;
    upsert_document_file_ocr_page(&mut db, document_file_id, 1, Some(ocr_text)).await?;

    cleanup_extra_pages(
        &mut db,
        document_file_id,
        &document_file.s3_prefix,
        1,
        prev_pages,
        state.clone(),
    )
    .await?;

    Ok(())
}

async fn cleanup_extra_pages(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
    s3_prefix: &str,
    pages: u32,
    prev_pages: u32,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    cleanup_extra_page_rows(db, document_file_id, pages).await?;

    let stale_keys = stale_page_image_keys(s3_prefix, prev_pages, pages);
    delete_s3_keys_best_effort(document_file_id, &stale_keys, state).await;

    Ok(())
}

pub async fn cleanup_extra_page_rows(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
    pages: u32,
) -> JobResult<()> {
    let pages_i32 = i32::try_from(pages).unwrap_or(i32::MAX);

    diesel::delete(
        document_file_pages::table
            .filter(document_file_pages::document_file_id.eq(document_file_id))
            .filter(document_file_pages::page_number.gt(pages_i32)),
    )
    .execute(db)
    .await?;

    diesel::delete(
        document_file_ocr_pages::table
            .filter(document_file_ocr_pages::document_file_id.eq(document_file_id))
            .filter(document_file_ocr_pages::page_number.gt(pages_i32)),
    )
    .execute(db)
    .await?;

    Ok(())
}

pub fn stale_page_image_keys(s3_prefix: &str, prev_pages: u32, pages: u32) -> Vec<String> {
    if prev_pages <= pages {
        return Vec::new();
    }

    ((pages + 1)..=prev_pages)
        .map(|page| format!("{}/pages/{}.png", s3_prefix, page))
        .collect()
}

async fn delete_s3_keys_best_effort(
    document_file_id: i64,
    keys: &[String],
    state: Data<Arc<AppState>>,
) {
    for key in keys {
        if let Err(err) = delete_from_s3(&state.s3_client, state.s3_bucket.as_str(), key).await {
            tracing::warn!(
                document_file_id,
                s3_key = %key,
                error = %err,
                "failed to delete stale page image from S3"
            );
        }
    }
}

async fn upsert_document_file_page(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
    page_number: i32,
    text_content: Option<String>,
) -> JobResult<()> {
    diesel::insert_into(document_file_pages::table)
        .values(&NewDocumentFilePage {
            document_file_id,
            page_number,
            text_content,
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
        .execute(db)
        .await?;

    Ok(())
}

async fn upsert_document_file_ocr_page(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
    page_number: i32,
    ocr_content: Option<String>,
) -> JobResult<()> {
    diesel::insert_into(document_file_ocr_pages::table)
        .values(&NewDocumentFileOcrPage {
            document_file_id,
            page_number,
            ocr_content,
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
        .execute(db)
        .await?;

    Ok(())
}

/**
 * Internal function to extract the text content of a specific page in a PDF document
 * by running `pdftotext` on the file and capturing the output.
 */
async fn extract_pdf_page_text(
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
async fn extract_pdf_page_image(
    file: String,
    page: u32,
    temp_dir: &Path,
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
    upload_png_to_s3(&output_path, &s3_key, state).await?;

    Ok(output_path)
}

pub(crate) async fn convert_image_to_png(
    file: String,
    output_path: &Path,
    _state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let input_path = format!("{}[0]", file);
    let output = Command::new("magick")
        .arg(&input_path)
        .arg("-auto-orient")
        .arg("-strip")
        .arg(output_path)
        .output()
        .await?;

    if !output.status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "magick failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        );
        return Err(error.into());
    }

    Ok(())
}

pub(crate) async fn upload_png_to_s3(
    output_path: &Path,
    s3_key: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let body = ByteStream::from_path(output_path).await?;
    upload_to_s3(
        &state.s3_client,
        state.s3_bucket.as_str(),
        s3_key,
        body,
        Some("image/png"),
    )
    .await?;

    Ok(())
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
