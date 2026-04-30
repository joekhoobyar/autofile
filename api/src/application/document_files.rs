use std::path::{Path, PathBuf};
use std::sync::Arc;

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use tokio::process::Command;
use uuid::Uuid;

use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::delete_from_s3;
use crate::infrastructure::s3::upload_file_to_s3;
use crate::schema::{document_file_ocr_pages, document_file_pages, document_files};
use crate::shared::app_state::AppState;
use crate::shared::util::{ApiError, JobResult, write_field_to_temp_file};

#[derive(Debug, Insertable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDocumentFile {
    pub document_id: i64,
    pub s3_prefix: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub created_by: i64,
    pub updated_by: i64,
}

#[derive(Debug)]
pub struct BufferedDocumentFileUpload {
    pub temp_path: std::path::PathBuf,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
}

#[derive(Clone, Debug)]
pub struct UploadedDocumentFile {
    pub s3_prefix: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
}

pub async fn buffer_document_file_field(
    field: &mut axum::extract::multipart::Field<'_>,
) -> Result<BufferedDocumentFileUpload, ApiError> {
    let mut filename = field
        .file_name()
        .ok_or_else(|| ApiError::bad_request("File field missing filename"))?
        .to_string();
    let content_type = field.content_type().map(|ct| ct.to_string());

    if filename == "_thumb.png" {
        filename = "thumb.png".to_string();
    }

    let temp_upload = write_field_to_temp_file(field).await?;
    Ok(BufferedDocumentFileUpload {
        temp_path: temp_upload.path,
        filename,
        content_type,
        size: temp_upload.size,
    })
}

pub async fn cleanup_buffered_document_file_upload(upload: &BufferedDocumentFileUpload) {
    let _ = tokio::fs::remove_file(&upload.temp_path).await;
}

pub async fn upload_document_file_to_s3(
    state: &AppState,
    upload: BufferedDocumentFileUpload,
) -> Result<UploadedDocumentFile, ApiError> {
    let s3_prefix = Uuid::new_v4().to_string();
    let s3_key = format!("{}/{}", s3_prefix, upload.filename);
    let upload_result = upload_file_to_s3(
        &state.s3_client,
        &state.s3_bucket,
        &s3_key,
        &upload.temp_path,
        upload.size,
        upload.content_type.as_deref(),
    )
    .await;
    let _ = tokio::fs::remove_file(&upload.temp_path).await;
    upload_result
        .map_err(|e| ApiError::internal_server_error(&format!("S3 upload failed: {}", e)))?;

    Ok(UploadedDocumentFile {
        s3_prefix,
        filename: upload.filename,
        content_type: upload.content_type,
        size: upload.size,
    })
}

pub async fn delete_uploaded_document_file_from_s3(
    state: &AppState,
    upload: &UploadedDocumentFile,
) {
    let s3_key = format!("{}/{}", upload.s3_prefix, upload.filename);
    let _ = delete_from_s3(&state.s3_client, &state.s3_bucket, &s3_key).await;
}

pub async fn insert_document_file(
    db: &mut AsyncPgConnection,
    document_id: i64,
    upload: UploadedDocumentFile,
    user_id: i64,
) -> Result<DocumentFile, diesel::result::Error> {
    diesel::insert_into(document_files::table)
        .values(&NewDocumentFile {
            document_id,
            s3_prefix: upload.s3_prefix,
            filename: upload.filename,
            content_type: upload.content_type,
            size: upload.size,
            created_by: user_id,
            updated_by: user_id,
        })
        .returning(DocumentFile::as_returning())
        .get_result(db)
        .await
}

#[derive(Clone, Debug)]
struct ProcessCommandOutput {
    success: bool,
    status: String,
    stderr: String,
}

#[async_trait::async_trait]
trait ProcessRunner {
    async fn run(&self, program: &str, args: &[String]) -> std::io::Result<ProcessCommandOutput>;
}

struct TokioProcessRunner;

#[async_trait::async_trait]
impl ProcessRunner for TokioProcessRunner {
    async fn run(&self, program: &str, args: &[String]) -> std::io::Result<ProcessCommandOutput> {
        let output = Command::new(program).args(args).output().await?;

        Ok(ProcessCommandOutput {
            success: output.status.success(),
            status: output.status.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

async fn run_process<R: ProcessRunner + ?Sized>(
    runner: &R,
    program: &str,
    args: &[String],
    tool_name: &str,
) -> JobResult<()> {
    let output = runner.run(program, args).await?;
    if !output.success {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!(
                "{tool_name} failed with status {}: {}",
                output.status, output.stderr
            ),
        );
        return Err(error.into());
    }

    Ok(())
}

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
    let mut document_file = document_files::table
        .find(document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await?;
    persist_document_file_content_type_fallback(&mut db, &mut document_file).await?;

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
            DocumentFileContentType::Csv => {
                process_file_pages_csv(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::Tsv => {
                process_file_pages_tsv(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state,
                )
                .await
            }
            DocumentFileContentType::OfficeDocument => {
                process_file_pages_office_document(
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
    Csv,
    Tsv,
    OfficeDocument,
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

    if content_type == "text/csv" || (content_type == "text/plain" && filename.ends_with(".csv")) {
        return Ok(DocumentFileContentType::Csv);
    }

    if content_type == "text/tab-separated-values"
        || (content_type == "text/plain" && filename.ends_with(".tsv"))
    {
        return Ok(DocumentFileContentType::Tsv);
    }

    if content_type == "application/msword"
        || content_type == "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        || content_type == "application/vnd.oasis.opendocument.text"
        || content_type == "application/vnd.ms-excel"
        || content_type == "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        || content_type == "application/vnd.oasis.opendocument.spreadsheet"
        || content_type == "application/vnd.ms-powerpoint"
        || content_type
            == "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        || content_type == "application/vnd.oasis.opendocument.presentation"
        || content_type == "application/mspowerpoint"
        || content_type == "application/powerpoint"
        || ((content_type == "application/octet-stream" || content_type == "text/plain")
            && (filename.ends_with(".doc")
                || filename.ends_with(".docx")
                || filename.ends_with(".odt")
                || filename.ends_with(".xls")
                || filename.ends_with(".xlsx")
                || filename.ends_with(".ods")
                || filename.ends_with(".ppt")
                || filename.ends_with(".pptx")
                || filename.ends_with(".odp")))
    {
        return Ok(DocumentFileContentType::OfficeDocument);
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

pub(crate) async fn persist_document_file_content_type_fallback(
    db: &mut diesel_async::AsyncPgConnection,
    document_file: &mut DocumentFile,
) -> JobResult<()> {
    let fallback_content_type = match fallback_document_file_content_type(&document_file.filename) {
        Some(fallback_content_type) => fallback_content_type,
        None => return Ok(()),
    };

    match document_file.content_type.as_deref() {
        None => {
            diesel::update(
                document_files::table
                    .find(document_file.id)
                    .filter(document_files::content_type.is_null()),
            )
            .set(document_files::content_type.eq(Some(fallback_content_type)))
            .execute(db)
            .await?;
        }
        Some("application/octet-stream" | "text/plain")
            if document_file.content_type.as_deref() != Some(fallback_content_type) =>
        {
            diesel::update(
                document_files::table
                    .find(document_file.id)
                    .filter(document_files::content_type.eq(document_file.content_type.as_deref())),
            )
            .set(document_files::content_type.eq(Some(fallback_content_type)))
            .execute(db)
            .await?;
        }
        _ => return Ok(()),
    }

    document_file.content_type = Some(fallback_content_type.to_string());
    Ok(())
}

fn fallback_document_file_content_type(filename: &str) -> Option<&'static str> {
    let extension = Path::new(filename)
        .extension()
        .and_then(|extension| extension.to_str())?
        .to_ascii_lowercase();

    match extension.as_str() {
        "pdf" => Some("application/pdf"),
        "jpg" | "jpeg" | "jfif" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "tif" | "tiff" => Some("image/tiff"),
        "svg" => Some("image/svg+xml"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "heic" => Some("image/heic"),
        "heif" => Some("image/heif"),
        "avif" => Some("image/avif"),
        "ico" => Some("image/x-icon"),
        "md" | "markdown" => Some("text/markdown"),
        "csv" => Some("text/csv"),
        "tsv" => Some("text/tab-separated-values"),
        "doc" => Some("application/msword"),
        "docx" => Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
        "odt" => Some("application/vnd.oasis.opendocument.text"),
        "xls" => Some("application/vnd.ms-excel"),
        "xlsx" => Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
        "ods" => Some("application/vnd.oasis.opendocument.spreadsheet"),
        "ppt" => Some("application/vnd.ms-powerpoint"),
        "pptx" => Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
        "odp" => Some("application/vnd.oasis.opendocument.presentation"),
        "html" | "htm" => Some("text/html"),
        "xhtml" => Some("application/xhtml+xml"),
        "txt" | "text" => Some("text/plain"),
        _ => None,
    }
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
    let runner = TokioProcessRunner;
    convert_plaintext_to_pdf_with_runner(text_file, &runner).await
}

async fn convert_plaintext_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    text_file: &str,
    runner: &R,
) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", text_file);

    let args = vec![
        "-f".to_string(),
        "markdown".to_string(),
        text_file.to_string(),
        "-o".to_string(),
        pdf_file.clone(),
        "--pdf-engine=xelatex".to_string(),
    ];
    run_process(runner, "pandoc", &args, "pandoc").await?;

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
 * Internal function to extract the images and text from a CSV document
 * by running `pandoc` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_csv(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file = convert_csv_to_pdf(temp_file).await?;

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
 * Internal function to convert a CSV file to PDF by running `pandoc`.
 */
pub(crate) async fn convert_csv_to_pdf(csv_file: &str) -> JobResult<String> {
    let runner = TokioProcessRunner;
    convert_csv_to_pdf_with_runner(csv_file, &runner).await
}

async fn convert_csv_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    csv_file: &str,
    runner: &R,
) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", csv_file);

    let args = vec![
        "-f".to_string(),
        "csv".to_string(),
        csv_file.to_string(),
        "-o".to_string(),
        pdf_file.clone(),
        "--pdf-engine=xelatex".to_string(),
    ];
    run_process(runner, "pandoc", &args, "pandoc").await?;

    Ok(pdf_file)
}

/**
 * Internal function to extract the images and text from a TSV document
 * by running `pandoc` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_tsv(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file = convert_tsv_to_pdf(temp_file).await?;

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
 * Internal function to extract the images and text from an office document
 * by running `soffice` on the file, converting to a PDF, then running process_file_pages_pdf().
 */
async fn process_file_pages_office_document(
    document_file_id: i64,
    document_file: &DocumentFile,
    temp_dir: &Path,
    temp_file: &str,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let pdf_file =
        convert_office_document_to_pdf(temp_file, document_file.filename.as_str()).await?;

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
 * Internal function to convert an office document file to PDF by running `soffice`.
 */
pub(crate) async fn convert_office_document_to_pdf(
    source_file: &str,
    original_filename: &str,
) -> JobResult<String> {
    let runner = TokioProcessRunner;
    convert_office_document_to_pdf_with_runner(source_file, original_filename, &runner).await
}

async fn convert_office_document_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    source_file: &str,
    original_filename: &str,
    runner: &R,
) -> JobResult<String> {
    let source_path = Path::new(source_file);
    let source_dir = source_path.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Cannot determine parent directory for {source_file}"),
        )
    })?;
    let source_name = Path::new(original_filename)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid original filename: {original_filename}"),
            )
        })?;

    let office_input = source_dir.join(source_name);
    let copied_input = office_input != source_path;
    if copied_input {
        tokio::fs::copy(source_path, &office_input).await?;
    }

    let convert_dir = source_dir.join(format!("soffice-out-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&convert_dir).await?;

    let conversion_result = async {
        let soffice_input = if copied_input {
            office_input.as_path()
        } else {
            source_path
        };

        let args = vec![
            "--headless".to_string(),
            "--convert-to".to_string(),
            "pdf".to_string(),
            "--outdir".to_string(),
            convert_dir.to_string_lossy().to_string(),
            soffice_input.to_string_lossy().to_string(),
        ];

        run_process(runner, "soffice", &args, "soffice").await?;

        let output_stem = soffice_input.file_stem().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Cannot derive output stem from {}", soffice_input.display()),
            )
        })?;

        let converted_pdf = convert_dir.join(format!("{}.pdf", output_stem.to_string_lossy()));
        if !tokio::fs::try_exists(&converted_pdf).await? {
            let error = std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "soffice did not produce expected output at {}",
                    converted_pdf.display()
                ),
            );
            return Err(error.into());
        }

        let final_pdf = format!("{}.pdf", source_file);
        tokio::fs::rename(&converted_pdf, final_pdf.as_str()).await?;
        Ok(final_pdf)
    }
    .await;

    if copied_input {
        if let Err(err) = tokio::fs::remove_file(&office_input).await {
            tracing::warn!(error = %err, path = %office_input.display(), "failed to remove soffice temp input");
        }
    }
    if let Err(err) = tokio::fs::remove_dir_all(&convert_dir).await {
        tracing::warn!(error = %err, path = %convert_dir.display(), "failed to remove soffice temp dir");
    }

    conversion_result
}

/**
 * Internal function to convert a TSV file to PDF by running `pandoc`.
 */
pub(crate) async fn convert_tsv_to_pdf(tsv_file: &str) -> JobResult<String> {
    let runner = TokioProcessRunner;
    convert_tsv_to_pdf_with_runner(tsv_file, &runner).await
}

async fn convert_tsv_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    tsv_file: &str,
    runner: &R,
) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", tsv_file);

    let args = vec![
        "-f".to_string(),
        "tsv".to_string(),
        tsv_file.to_string(),
        "-o".to_string(),
        pdf_file.clone(),
        "--pdf-engine=xelatex".to_string(),
    ];
    run_process(runner, "pandoc", &args, "pandoc").await?;

    Ok(pdf_file)
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
    let runner = TokioProcessRunner;
    convert_html_to_pdf_with_runner(html_file, &runner).await
}

async fn convert_html_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    html_file: &str,
    runner: &R,
) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", html_file);

    let args = vec![html_file.to_string(), pdf_file.clone()];
    run_process(runner, "weasyprint", &args, "weasyprint").await?;

    Ok(pdf_file)
}

/**
 * Internal function to convert a markdown file to PDF by running `pandoc`.
 */
pub(crate) async fn convert_markdown_to_pdf(markdown_file: &str) -> JobResult<String> {
    let runner = TokioProcessRunner;
    convert_markdown_to_pdf_with_runner(markdown_file, &runner).await
}

async fn convert_markdown_to_pdf_with_runner<R: ProcessRunner + ?Sized>(
    markdown_file: &str,
    runner: &R,
) -> JobResult<String> {
    let pdf_file = format!("{}.pdf", markdown_file);

    let args = vec![
        "-f".to_string(),
        "markdown".to_string(),
        markdown_file.to_string(),
        "-o".to_string(),
        pdf_file.clone(),
        "--pdf-engine=xelatex".to_string(),
    ];
    run_process(runner, "pandoc", &args, "pandoc").await?;

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
    let size = tokio::fs::metadata(output_path).await?.len();
    upload_file_to_s3(
        &state.s3_client,
        state.s3_bucket.as_str(),
        s3_key,
        output_path,
        i64::try_from(size)?,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Clone)]
    enum FakeRunnerMode {
        SuccessCreatesOutput,
        SuccessNoOutput,
        Failure { status: String, stderr: String },
    }

    struct FakeProcessRunner {
        mode: FakeRunnerMode,
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeProcessRunner {
        fn new(mode: FakeRunnerMode) -> Self {
            Self {
                mode,
                calls: Mutex::new(Vec::new()),
            }
        }

        fn calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().expect("calls lock poisoned").clone()
        }
    }

    #[async_trait::async_trait]
    impl ProcessRunner for FakeProcessRunner {
        async fn run(
            &self,
            program: &str,
            args: &[String],
        ) -> std::io::Result<ProcessCommandOutput> {
            self.calls
                .lock()
                .expect("calls lock poisoned")
                .push((program.to_string(), args.to_vec()));

            match &self.mode {
                FakeRunnerMode::SuccessCreatesOutput => {
                    let outdir = arg_value(args, "--outdir").expect("--outdir not passed");
                    let input = args.last().expect("input path not passed");
                    let input_stem = Path::new(input)
                        .file_stem()
                        .expect("input stem missing")
                        .to_string_lossy();
                    let output_path = Path::new(outdir).join(format!("{}.pdf", input_stem));
                    tokio::fs::write(output_path, b"pdf").await?;

                    Ok(ProcessCommandOutput {
                        success: true,
                        status: "0".to_string(),
                        stderr: String::new(),
                    })
                }
                FakeRunnerMode::SuccessNoOutput => Ok(ProcessCommandOutput {
                    success: true,
                    status: "0".to_string(),
                    stderr: String::new(),
                }),
                FakeRunnerMode::Failure { status, stderr } => Ok(ProcessCommandOutput {
                    success: false,
                    status: status.clone(),
                    stderr: stderr.clone(),
                }),
            }
        }
    }

    fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
        let idx = args.iter().position(|arg| arg == name)?;
        args.get(idx + 1).map(String::as_str)
    }

    fn assert_single_process_call(
        runner: &FakeProcessRunner,
        expected_program: &str,
        expected_args: &[&str],
    ) {
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        let (program, args) = &calls[0];
        assert_eq!(program, expected_program);
        let expected: Vec<String> = expected_args.iter().map(|s| s.to_string()).collect();
        assert_eq!(*args, expected);
    }

    async fn create_test_source_file() -> (PathBuf, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");

        let source_path = test_dir.join("staged-file");
        tokio::fs::write(&source_path, b"dummy")
            .await
            .expect("failed to write source file");

        (test_dir, source_path)
    }

    #[test]
    fn fallback_content_type_maps_supported_extensions() {
        assert_eq!(
            fallback_document_file_content_type("report.pdf"),
            Some("application/pdf")
        );
        assert_eq!(
            fallback_document_file_content_type("notes.md"),
            Some("text/markdown")
        );
        assert_eq!(
            fallback_document_file_content_type("data.csv"),
            Some("text/csv")
        );
        assert_eq!(
            fallback_document_file_content_type("data.tsv"),
            Some("text/tab-separated-values")
        );
        assert_eq!(
            fallback_document_file_content_type("report.docx"),
            Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document")
        );
        assert_eq!(
            fallback_document_file_content_type("page.html"),
            Some("text/html")
        );
        assert_eq!(
            fallback_document_file_content_type("notes.txt"),
            Some("text/plain")
        );
    }

    #[test]
    fn fallback_content_type_maps_common_image_extensions() {
        for (extension, expected_content_type) in [
            ("jpg", "image/jpeg"),
            ("jpeg", "image/jpeg"),
            ("png", "image/png"),
            ("tif", "image/tiff"),
            ("tiff", "image/tiff"),
            ("svg", "image/svg+xml"),
            ("gif", "image/gif"),
            ("webp", "image/webp"),
            ("bmp", "image/bmp"),
            ("heic", "image/heic"),
            ("heif", "image/heif"),
            ("avif", "image/avif"),
            ("ico", "image/x-icon"),
            ("jfif", "image/jpeg"),
        ] {
            assert_eq!(
                fallback_document_file_content_type(&format!("image.{extension}")),
                Some(expected_content_type)
            );
        }
    }

    #[test]
    fn fallback_content_type_is_case_insensitive() {
        assert_eq!(
            fallback_document_file_content_type("REPORT.PDF"),
            Some("application/pdf")
        );
        assert_eq!(
            fallback_document_file_content_type("IMAGE.PNG"),
            Some("image/png")
        );
    }

    #[test]
    fn parse_content_type_rejects_missing_content_type() {
        let err = parse_document_file_content_type(None, "archive.zip").unwrap_err();

        assert!(
            err.to_string()
                .contains("Document file is missing content_type")
        );
    }

    #[test]
    fn parse_content_type_handles_office_mime_types() {
        assert_eq!(
            parse_document_file_content_type(Some("application/msword"), "report.doc").unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.openxmlformats-officedocument.wordprocessingml.document"),
                "report.docx"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.oasis.opendocument.text"),
                "report.odt"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/vnd.ms-excel"), "report.xls")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"),
                "report.xlsx"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.oasis.opendocument.spreadsheet"),
                "report.ods"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/vnd.ms-powerpoint"), "slides.ppt")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.openxmlformats-officedocument.presentationml.presentation"),
                "slides.pptx"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(
                Some("application/vnd.oasis.opendocument.presentation"),
                "slides.odp"
            )
            .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/mspowerpoint"), "slides.ppt")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/powerpoint"), "slides.ppt").unwrap(),
            DocumentFileContentType::OfficeDocument
        );
    }

    #[test]
    fn parse_content_type_handles_office_extension_fallbacks() {
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "report.doc")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "report.docx")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("text/plain"), "report.odt").unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "report.xls")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "report.xlsx")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("text/plain"), "report.ods").unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "slides.ppt")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("application/octet-stream"), "slides.pptx")
                .unwrap(),
            DocumentFileContentType::OfficeDocument
        );
        assert_eq!(
            parse_document_file_content_type(Some("text/plain"), "slides.odp").unwrap(),
            DocumentFileContentType::OfficeDocument
        );
    }

    #[tokio::test]
    async fn plaintext_conversion_builds_expected_pandoc_command() {
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let pdf = convert_plaintext_to_pdf_with_runner("/tmp/input.txt", &runner)
            .await
            .expect("conversion should succeed");
        assert_eq!(pdf, "/tmp/input.txt.pdf");

        assert_single_process_call(
            &runner,
            "pandoc",
            &[
                "-f",
                "markdown",
                "/tmp/input.txt",
                "-o",
                "/tmp/input.txt.pdf",
                "--pdf-engine=xelatex",
            ],
        );
    }

    #[tokio::test]
    async fn markdown_conversion_builds_expected_pandoc_command() {
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let pdf = convert_markdown_to_pdf_with_runner("/tmp/input.md", &runner)
            .await
            .expect("conversion should succeed");
        assert_eq!(pdf, "/tmp/input.md.pdf");

        assert_single_process_call(
            &runner,
            "pandoc",
            &[
                "-f",
                "markdown",
                "/tmp/input.md",
                "-o",
                "/tmp/input.md.pdf",
                "--pdf-engine=xelatex",
            ],
        );
    }

    #[tokio::test]
    async fn csv_conversion_builds_expected_pandoc_command() {
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let pdf = convert_csv_to_pdf_with_runner("/tmp/input.csv", &runner)
            .await
            .expect("conversion should succeed");
        assert_eq!(pdf, "/tmp/input.csv.pdf");

        assert_single_process_call(
            &runner,
            "pandoc",
            &[
                "-f",
                "csv",
                "/tmp/input.csv",
                "-o",
                "/tmp/input.csv.pdf",
                "--pdf-engine=xelatex",
            ],
        );
    }

    #[tokio::test]
    async fn tsv_conversion_builds_expected_pandoc_command() {
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let pdf = convert_tsv_to_pdf_with_runner("/tmp/input.tsv", &runner)
            .await
            .expect("conversion should succeed");
        assert_eq!(pdf, "/tmp/input.tsv.pdf");

        assert_single_process_call(
            &runner,
            "pandoc",
            &[
                "-f",
                "tsv",
                "/tmp/input.tsv",
                "-o",
                "/tmp/input.tsv.pdf",
                "--pdf-engine=xelatex",
            ],
        );
    }

    #[tokio::test]
    async fn html_conversion_builds_expected_weasyprint_command() {
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let pdf = convert_html_to_pdf_with_runner("/tmp/input.html", &runner)
            .await
            .expect("conversion should succeed");
        assert_eq!(pdf, "/tmp/input.html.pdf");

        assert_single_process_call(
            &runner,
            "weasyprint",
            &["/tmp/input.html", "/tmp/input.html.pdf"],
        );
    }

    #[tokio::test]
    async fn pandoc_and_weasyprint_conversion_include_stderr_on_failure() {
        let pandoc_runner = FakeProcessRunner::new(FakeRunnerMode::Failure {
            status: "2".to_string(),
            stderr: "pandoc broke".to_string(),
        });
        let pandoc_err = convert_csv_to_pdf_with_runner("/tmp/input.csv", &pandoc_runner)
            .await
            .expect_err("conversion should fail");
        assert!(
            pandoc_err
                .to_string()
                .contains("pandoc failed with status 2")
        );
        assert!(pandoc_err.to_string().contains("pandoc broke"));

        let weasy_runner = FakeProcessRunner::new(FakeRunnerMode::Failure {
            status: "3".to_string(),
            stderr: "weasy broke".to_string(),
        });
        let weasy_err = convert_html_to_pdf_with_runner("/tmp/input.html", &weasy_runner)
            .await
            .expect_err("conversion should fail");
        assert!(
            weasy_err
                .to_string()
                .contains("weasyprint failed with status 3")
        );
        assert!(weasy_err.to_string().contains("weasy broke"));
    }

    #[tokio::test]
    async fn office_conversion_builds_soffice_command_and_moves_output() {
        let (test_dir, source_path) = create_test_source_file().await;
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessCreatesOutput);

        let result = convert_office_document_to_pdf_with_runner(
            source_path.to_string_lossy().as_ref(),
            "invoice.docx",
            &runner,
        )
        .await
        .expect("conversion should succeed");

        assert_eq!(result, format!("{}.pdf", source_path.to_string_lossy()));
        assert!(
            tokio::fs::try_exists(Path::new(result.as_str()))
                .await
                .expect("try_exists failed")
        );

        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        let (program, args) = &calls[0];
        assert_eq!(program, "soffice");
        assert_eq!(args[0], "--headless");
        assert_eq!(args[1], "--convert-to");
        assert_eq!(args[2], "pdf");
        assert_eq!(args[3], "--outdir");
        assert!(args[4].contains("soffice-out-"));
        assert!(args[5].ends_with("invoice.docx"));

        let temp_outdir = Path::new(&args[4]);
        assert!(
            !tokio::fs::try_exists(temp_outdir)
                .await
                .expect("try_exists failed")
        );
        assert!(
            !tokio::fs::try_exists(&test_dir.join("invoice.docx"))
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn office_conversion_includes_soffice_stderr_on_failure() {
        let (test_dir, source_path) = create_test_source_file().await;
        let runner = FakeProcessRunner::new(FakeRunnerMode::Failure {
            status: "1".to_string(),
            stderr: "conversion failed".to_string(),
        });

        let err = convert_office_document_to_pdf_with_runner(
            source_path.to_string_lossy().as_ref(),
            "invoice.docx",
            &runner,
        )
        .await
        .expect_err("conversion should fail");
        let err_text = err.to_string();

        assert!(err_text.contains("soffice failed with status 1"));
        assert!(err_text.contains("conversion failed"));

        let calls = runner.calls();
        let outdir = arg_value(&calls[0].1, "--outdir").expect("missing --outdir value");
        assert!(
            !tokio::fs::try_exists(Path::new(outdir))
                .await
                .expect("try_exists failed")
        );
        assert!(
            !tokio::fs::try_exists(&test_dir.join("invoice.docx"))
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn office_conversion_fails_when_soffice_does_not_create_output_file() {
        let (test_dir, source_path) = create_test_source_file().await;
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessNoOutput);

        let err = convert_office_document_to_pdf_with_runner(
            source_path.to_string_lossy().as_ref(),
            "invoice.docx",
            &runner,
        )
        .await
        .expect_err("conversion should fail");

        assert!(
            err.to_string()
                .contains("soffice did not produce expected output")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }
}
