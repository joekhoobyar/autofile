use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use apalis::prelude::*;
use apalis_redis::RedisStorage;
use bb8::PooledConnection;
use chrono::Utc;
use diesel::dsl::select;
use diesel::prelude::*;
use diesel::result::{DatabaseErrorKind, Error as DieselError};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use tokio::io::AsyncRead;
use uuid::Uuid;

use crate::application::app_settings::get_app_settings;
use crate::application::jobs::{FastJob, MediumJob};
use crate::application::malware_scanning::{ScanMetadata, ScanOutcome};
use crate::domain::document_files::{
    DocumentFile, DocumentFileOcrPage, DocumentFilePage, DocumentFileView, SCAN_STATUS_CLEAN,
    SCAN_STATUS_ERROR, SCAN_STATUS_INFECTED, SCAN_STATUS_NOT_REQUIRED, SCAN_STATUS_PENDING,
    SCAN_STATUS_SCANNING,
};
use crate::domain::document_types::UNSPECIFIED_DOCUMENT_TYPE_ID;
use crate::domain::users::SYSTEM_USER_ID;
use crate::infrastructure::s3::delete_from_s3;
use crate::infrastructure::s3::delete_prefix_from_s3;
use crate::infrastructure::s3::upload_file_to_s3;
use crate::schema::{document_file_ocr_pages, document_file_pages, document_files, documents};
use crate::shared::app_state::AppState;
use crate::shared::config::MalwareScannerFailurePolicy;
use crate::shared::errors::{ApiError, JobResult};
use crate::shared::process::sanitized_command;
use crate::shared::uploads::write_field_to_temp_file;

const PANDOC_PDF_ENGINE: &str = "--pdf-engine=weasyprint";

#[derive(Debug, Insertable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewDocumentFile {
    pub document_id: i64,
    pub s3_prefix: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub checksum_sha256: String,
    pub scan_status: String,
    pub scan_requested: bool,
    pub scan_requested_by: Option<i64>,
    pub created_by: i64,
    pub updated_by: i64,
}

#[derive(Debug)]
pub struct BufferedDocumentFileUpload {
    pub temp_path: std::path::PathBuf,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub checksum_sha256: String,
}

#[derive(Clone, Debug)]
pub struct UploadedDocumentFile {
    pub s3_prefix: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub size: i64,
    pub checksum_sha256: String,
}

#[derive(Debug, Clone, Copy)]
pub struct UploadScanDecision {
    pub requested: bool,
}

impl UploadScanDecision {
    fn scan_status(self) -> &'static str {
        if self.requested {
            SCAN_STATUS_PENDING
        } else {
            SCAN_STATUS_NOT_REQUIRED
        }
    }
}

pub struct DocumentFileDownloadMetadata {
    pub s3_key: String,
    pub filename: String,
    pub content_type: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn unavailable_file_api_error(file: &DocumentFile) -> ApiError {
    ApiError::conflict(
        file.content_availability()
            .unavailable_message()
            .unwrap_or("File content is unavailable"),
    )
}

fn document_file_view(file: DocumentFile) -> DocumentFileView {
    DocumentFileView {
        id: file.id,
        document_id: file.document_id,
        filename: file.filename,
        content_type: file.content_type,
        size: file.size,
        pages: file.pages,
        scan_status: file.scan_status,
        scan_requested: file.scan_requested,
        scan_scanner: file.scan_scanner,
        scan_scanner_version: file.scan_scanner_version,
        scan_signature_version: file.scan_signature_version,
        scan_threat_name: file.scan_threat_name,
        scan_started_at: file.scan_started_at,
        scan_completed_at: file.scan_completed_at,
        content_available: file.content_available,
        created_at: file.created_at,
        created_by: file.created_by,
        updated_at: file.updated_at,
        updated_by: file.updated_by,
    }
}

pub async fn list_document_files(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
) -> Result<Vec<DocumentFileView>, ApiError> {
    document_files::table
        .filter(document_files::document_id.eq(document_id))
        .select(DocumentFileView::as_select())
        .order(document_files::id.asc())
        .load::<DocumentFileView>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_files", e))
}

pub async fn get_document_file(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    id: i64,
) -> Result<DocumentFileView, ApiError> {
    document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(DocumentFileView::as_select())
        .first::<DocumentFileView>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document_file", e))
}

pub async fn get_document_file_thumbnail_metadata(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    id: i64,
) -> Result<(String, chrono::DateTime<chrono::Utc>), ApiError> {
    let file = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document file thumbnail", e))?;

    if !file.content_available() {
        return Err(unavailable_file_api_error(&file));
    }

    Ok((format!("{}/_thumb.png", file.s3_prefix), file.updated_at))
}

pub async fn get_document_file_download_metadata(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    id: i64,
) -> Result<DocumentFileDownloadMetadata, ApiError> {
    let file = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document file download", e))?;

    if !file.content_available() {
        return Err(unavailable_file_api_error(&file));
    }

    Ok(DocumentFileDownloadMetadata {
        s3_key: format!("{}/{}", file.s3_prefix, file.filename),
        filename: file.filename,
        content_type: file.content_type,
        updated_at: file.updated_at,
    })
}

pub async fn ensure_document_file_available(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    id: i64,
) -> Result<(), ApiError> {
    let file = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(id))
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to fetch document file", e))?;

    if !file.content_available() {
        return Err(unavailable_file_api_error(&file));
    }

    Ok(())
}

pub async fn list_document_file_pages(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    document_file_id: i64,
) -> Result<Vec<DocumentFilePage>, ApiError> {
    ensure_document_file_available(db, document_id, document_file_id).await?;

    document_file_pages::table
        .inner_join(
            document_files::table.on(document_files::id.eq(document_file_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .filter(document_file_pages::document_file_id.eq(document_file_id))
        .select(DocumentFilePage::as_select())
        .order(document_file_pages::page_number.asc())
        .load::<DocumentFilePage>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_file_pages", e))
}

pub async fn list_document_file_ocr_pages(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    document_file_id: i64,
) -> Result<Vec<DocumentFileOcrPage>, ApiError> {
    ensure_document_file_available(db, document_id, document_file_id).await?;

    document_file_ocr_pages::table
        .inner_join(
            document_files::table
                .on(document_files::id.eq(document_file_ocr_pages::document_file_id)),
        )
        .filter(document_files::document_id.eq(document_id))
        .filter(document_file_ocr_pages::document_file_id.eq(document_file_id))
        .select(DocumentFileOcrPage::as_select())
        .order(document_file_ocr_pages::page_number.asc())
        .load::<DocumentFileOcrPage>(db)
        .await
        .map_err(|e| ApiError::from_diesel("Failed to list document_file_ocr_pages", e))
}

pub async fn get_document_file_page_image_key(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    document_file_id: i64,
    page_number: i32,
) -> Result<String, ApiError> {
    let file = document_files::table
        .filter(document_files::document_id.eq(document_id))
        .filter(document_files::id.eq(document_file_id))
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(db)
        .await
        .map_err(|e| {
            if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document file not found")
            } else {
                ApiError::from_diesel("Failed to fetch document file", e)
            }
        })?;

    if !file.content_available() {
        return Err(unavailable_file_api_error(&file));
    }

    Ok(format!("{}/pages/{}.png", file.s3_prefix, page_number))
}

pub async fn create_document_file(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    document_id: i64,
    file_upload: BufferedDocumentFileUpload,
    virus_scan: Option<bool>,
) -> Result<DocumentFileView, ApiError> {
    let scan_decision = resolve_upload_scan_decision(db, virus_scan).await?;
    let file_info = upload_document_file_to_s3(&state, file_upload).await?;
    let file_info_for_cleanup = file_info.clone();

    let fast_jobs = state.fast_jobs.as_ref().clone();
    let medium_jobs = state.medium_jobs.as_ref().clone();
    let scan_enqueue_failed = Arc::new(AtomicBool::new(false));
    let scan_enqueue_failed_for_tx = Arc::clone(&scan_enqueue_failed);
    let thumbnail_enqueue_failed = Arc::new(AtomicBool::new(false));
    let thumbnail_enqueue_failed_for_tx = Arc::clone(&thumbnail_enqueue_failed);
    let pages_enqueue_failed = Arc::new(AtomicBool::new(false));
    let pages_enqueue_failed_for_tx = Arc::clone(&pages_enqueue_failed);

    let result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(async move |conn| {
            let mut fast_jobs = fast_jobs;
            let mut medium_jobs = medium_jobs;

            documents::table
                .find(document_id)
                .select(documents::id)
                .first::<i64>(conn)
                .await?;

            let inserted_file =
                insert_document_file(conn, document_id, file_info, user_id, scan_decision).await?;

            if scan_decision.requested {
                if medium_jobs
                    .push(MediumJob::ScanDocumentFile {
                        document_file_id: inserted_file.id,
                    })
                    .await
                    .is_err()
                {
                    scan_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                    return Err(diesel::result::Error::RollbackTransaction);
                }
            } else if enqueue_post_upload_processing_jobs(
                &mut fast_jobs,
                &mut medium_jobs,
                inserted_file.id,
            )
            .await
            .is_err()
            {
                pages_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                thumbnail_enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                return Err(diesel::result::Error::RollbackTransaction);
            }

            Ok(document_file_view(inserted_file))
        })
        .await;

    match result {
        Ok(document_file) => Ok(document_file),
        Err(e) => {
            delete_uploaded_document_file_from_s3(&state, &file_info_for_cleanup).await;
            if matches!(e, diesel::result::Error::RollbackTransaction) {
                let scan_failed = scan_enqueue_failed.as_ref().load(Ordering::Relaxed);
                let pages_failed = pages_enqueue_failed.as_ref().load(Ordering::Relaxed);
                let thumbnail_failed = thumbnail_enqueue_failed.as_ref().load(Ordering::Relaxed);
                if scan_failed {
                    Err(ApiError::internal_server_error(
                        "Failed to enqueue virus scan job",
                    ))
                } else if pages_failed {
                    Err(ApiError::internal_server_error(
                        "Failed to enqueue file pages job",
                    ))
                } else if thumbnail_failed {
                    Err(ApiError::internal_server_error(
                        "Failed to enqueue thumbnail job",
                    ))
                } else {
                    Err(ApiError::from_diesel("Failed to create document_file", e))
                }
            } else {
                Err(ApiError::from_diesel("Failed to create document_file", e))
            }
        }
    }
}

pub async fn delete_document_file(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    document_id: i64,
    id: i64,
) -> Result<(), ApiError> {
    let delete_last_file = Arc::new(AtomicBool::new(false));
    let delete_last_file_for_tx = Arc::clone(&delete_last_file);

    let deleted_prefix = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(async move |conn| {
            let files = document_files::table
                .filter(document_files::document_id.eq(document_id))
                .select(DocumentFile::as_select())
                .order(document_files::id.asc())
                .load::<DocumentFile>(conn)
                .await?;

            let Some(file_index) = files.iter().position(|file| file.id == id) else {
                return Err(diesel::result::Error::NotFound);
            };

            if files.len() <= 1 {
                delete_last_file_for_tx.store(true, Ordering::Relaxed);
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let deleted_file = &files[file_index];
            let replacement_thumbnail = if file_index == 0 {
                files
                    .get(1)
                    .map(|next_file| format!("{}/_thumb.png", next_file.s3_prefix))
            } else {
                None
            };

            diesel::delete(
                document_file_ocr_pages::table
                    .filter(document_file_ocr_pages::document_file_id.eq(id)),
            )
            .execute(conn)
            .await?;

            diesel::delete(
                document_file_pages::table.filter(document_file_pages::document_file_id.eq(id)),
            )
            .execute(conn)
            .await?;

            let affected = diesel::delete(
                document_files::table
                    .filter(document_files::document_id.eq(document_id))
                    .filter(document_files::id.eq(id)),
            )
            .execute(conn)
            .await?;
            if affected == 0 {
                return Err(diesel::result::Error::NotFound);
            }

            if let Some(thumbnail_key) = replacement_thumbnail {
                diesel::update(documents::table.filter(documents::id.eq(document_id)))
                    .set(documents::s3_thumbnail.eq(thumbnail_key))
                    .execute(conn)
                    .await?;
            }

            Ok(deleted_file.s3_prefix.clone())
        })
        .await
        .map_err(|e| {
            if delete_last_file.as_ref().load(Ordering::Relaxed) {
                ApiError::conflict("Cannot delete the last file in a document")
            } else if matches!(e, diesel::result::Error::NotFound) {
                ApiError::not_found("Document file not found")
            } else {
                ApiError::from_diesel("Failed to delete document_file", e)
            }
        })?;

    let delete_prefix = format!("{}/", deleted_prefix);
    delete_prefix_from_s3(&state.s3_client, &state.s3_bucket, &delete_prefix)
        .await
        .map_err(|e| {
            ApiError::internal_server_error(&format!(
                "Failed to delete document file from storage: {}",
                e
            ))
        })?;

    Ok(())
}

pub async fn rescan_document_file(
    state: Arc<AppState>,
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    user_id: i64,
    document_id: i64,
    id: i64,
) -> Result<DocumentFileView, ApiError> {
    let settings = get_app_settings(db).await?;
    if !settings.virus_scanning_enabled {
        return Err(ApiError::conflict("Virus scanning is disabled"));
    }

    let medium_jobs = state.medium_jobs.as_ref().clone();
    let enqueue_failed = Arc::new(AtomicBool::new(false));
    let enqueue_failed_for_tx = Arc::clone(&enqueue_failed);

    let result = db
        .build_transaction()
        .run::<_, diesel::result::Error, _>(async move |conn| {
            let current = document_files::table
                .filter(document_files::document_id.eq(document_id))
                .filter(document_files::id.eq(id))
                .select(DocumentFile::as_select())
                .first::<DocumentFile>(conn)
                .await?;

            if matches!(
                current.scan_status.as_str(),
                SCAN_STATUS_PENDING | SCAN_STATUS_SCANNING
            ) {
                return Err(diesel::result::Error::RollbackTransaction);
            }

            let updated = diesel::update(document_files::table.filter(document_files::id.eq(id)))
                .set((
                    document_files::scan_status.eq(SCAN_STATUS_PENDING),
                    document_files::scan_requested.eq(true),
                    document_files::scan_requested_by.eq(Some(user_id)),
                    document_files::scan_scanner.eq::<Option<String>>(None),
                    document_files::scan_scanner_version.eq::<Option<String>>(None),
                    document_files::scan_signature_version.eq::<Option<String>>(None),
                    document_files::scan_threat_name.eq::<Option<String>>(None),
                    document_files::scan_error.eq::<Option<String>>(None),
                    document_files::scan_started_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                    document_files::scan_completed_at.eq::<Option<chrono::DateTime<Utc>>>(None),
                    document_files::updated_by.eq(user_id),
                    document_files::updated_at.eq(Utc::now()),
                ))
                .returning(DocumentFile::as_returning())
                .get_result::<DocumentFile>(conn)
                .await?;

            let mut medium_jobs = medium_jobs;
            if medium_jobs
                .push(MediumJob::ScanDocumentFile {
                    document_file_id: updated.id,
                })
                .await
                .is_err()
            {
                enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                return Err(diesel::result::Error::RollbackTransaction);
            }

            Ok(document_file_view(updated))
        })
        .await;

    result.map_err(|err| {
        if matches!(err, diesel::result::Error::NotFound) {
            ApiError::not_found("Document file not found")
        } else if enqueue_failed.as_ref().load(Ordering::Relaxed) {
            ApiError::internal_server_error("Failed to enqueue virus scan job")
        } else if matches!(err, diesel::result::Error::RollbackTransaction) {
            ApiError::conflict("File is already pending or actively scanning")
        } else {
            ApiError::from_diesel("Failed to rescan document file", err)
        }
    })
}

pub async fn buffer_document_file_field(
    field: &mut axum::extract::multipart::Field<'_>,
    max_bytes: u64,
) -> Result<BufferedDocumentFileUpload, ApiError> {
    let mut filename = field
        .file_name()
        .ok_or_else(|| ApiError::bad_request("File field missing filename"))?
        .to_string();
    let content_type = field.content_type().map(|ct| ct.to_string());

    if filename == "_thumb.png" {
        filename = "thumb.png".to_string();
    }

    let temp_upload = write_field_to_temp_file(field, max_bytes).await?;
    Ok(BufferedDocumentFileUpload {
        temp_path: temp_upload.path,
        filename,
        content_type,
        size: temp_upload.size,
        checksum_sha256: temp_upload.checksum_sha256,
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
        checksum_sha256: upload.checksum_sha256,
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
    scan_decision: UploadScanDecision,
) -> Result<DocumentFile, diesel::result::Error> {
    diesel::insert_into(document_files::table)
        .values(&NewDocumentFile {
            document_id,
            s3_prefix: upload.s3_prefix,
            filename: upload.filename,
            content_type: upload.content_type,
            size: upload.size,
            checksum_sha256: upload.checksum_sha256,
            scan_status: scan_decision.scan_status().to_string(),
            scan_requested: scan_decision.requested,
            scan_requested_by: scan_decision.requested.then_some(user_id),
            created_by: user_id,
            updated_by: user_id,
        })
        .returning(DocumentFile::as_returning())
        .get_result(db)
        .await
}

pub async fn resolve_upload_scan_decision(
    db: &mut PooledConnection<'_, AsyncDieselConnectionManager<AsyncPgConnection>>,
    virus_scan: Option<bool>,
) -> Result<UploadScanDecision, ApiError> {
    let settings = get_app_settings(db).await?;
    Ok(UploadScanDecision {
        requested: settings.virus_scanning_enabled
            && virus_scan.unwrap_or(settings.virus_scan_by_default),
    })
}

async fn enqueue_post_upload_processing_jobs(
    fast_jobs: &mut RedisStorage<FastJob>,
    medium_jobs: &mut RedisStorage<MediumJob>,
    document_file_id: i64,
) -> Result<(), ()> {
    medium_jobs
        .push(MediumJob::ProcessFilePages { document_file_id })
        .await
        .map_err(|_| ())?;
    fast_jobs
        .push(FastJob::GenerateThumbnail {
            document_file_id,
            page: 1,
            width: 800,
        })
        .await
        .map_err(|_| ())?;
    Ok(())
}

pub async fn scan_document_file(
    document_file_id: i64,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let mut db = state.db_pool.get().await?;
    let document_file = diesel::update(
        document_files::table
            .filter(document_files::id.eq(document_file_id))
            .filter(document_files::scan_status.eq_any([SCAN_STATUS_PENDING, SCAN_STATUS_ERROR])),
    )
    .set((
        document_files::scan_status.eq(SCAN_STATUS_SCANNING),
        document_files::scan_scanner.eq::<Option<String>>(None),
        document_files::scan_scanner_version.eq::<Option<String>>(None),
        document_files::scan_signature_version.eq::<Option<String>>(None),
        document_files::scan_threat_name.eq::<Option<String>>(None),
        document_files::scan_error.eq::<Option<String>>(None),
        document_files::scan_started_at.eq(Some(Utc::now())),
        document_files::scan_completed_at.eq::<Option<chrono::DateTime<Utc>>>(None),
        document_files::updated_at.eq(Utc::now()),
    ))
    .returning(DocumentFile::as_returning())
    .get_result::<DocumentFile>(&mut db)
    .await
    .optional()?;

    let Some(document_file) = document_file else {
        return Ok(());
    };

    let s3_key = format!("{}/{}", document_file.s3_prefix, document_file.filename);
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(&s3_key)
        .send()
        .await?;
    let mut reader = object.body.into_async_read();
    let metadata = ScanMetadata {
        filename: Some(document_file.filename.clone()),
        content_type: document_file.content_type.clone(),
        size_bytes: Some(document_file.size),
    };

    match state.malware_scanner.scan(&mut reader, metadata).await {
        Ok(ScanOutcome::Clean {
            scanner,
            scanner_version,
            signature_version,
        }) => {
            let mut fast_jobs = state.fast_jobs.as_ref().clone();
            let mut medium_jobs = state.medium_jobs.as_ref().clone();
            let enqueue_failed = Arc::new(AtomicBool::new(false));
            let enqueue_failed_for_tx = Arc::clone(&enqueue_failed);
            let result = db
                .build_transaction()
                .run::<_, diesel::result::Error, _>(async move |conn| {
                    diesel::update(
                        document_files::table.filter(document_files::id.eq(document_file_id)),
                    )
                    .set((
                        document_files::scan_status.eq(SCAN_STATUS_CLEAN),
                        document_files::scan_scanner.eq(Some(scanner)),
                        document_files::scan_scanner_version.eq(scanner_version),
                        document_files::scan_signature_version.eq(signature_version),
                        document_files::scan_threat_name.eq::<Option<String>>(None),
                        document_files::scan_error.eq::<Option<String>>(None),
                        document_files::scan_completed_at.eq(Some(Utc::now())),
                        document_files::updated_at.eq(Utc::now()),
                    ))
                    .execute(conn)
                    .await?;
                    enqueue_post_upload_processing_jobs(
                        &mut fast_jobs,
                        &mut medium_jobs,
                        document_file_id,
                    )
                    .await
                    .map_err(|_| {
                        enqueue_failed_for_tx.store(true, Ordering::Relaxed);
                        diesel::result::Error::RollbackTransaction
                    })?;
                    Ok(())
                })
                .await;

            if let Err(err) = result {
                if enqueue_failed.as_ref().load(Ordering::Relaxed) {
                    mark_scan_error(
                        &mut db,
                        document_file_id,
                        "autofile",
                        "Failed to enqueue document processing jobs",
                    )
                    .await?;
                    return Err(std::io::Error::other(
                        "Failed to enqueue document processing jobs",
                    )
                    .into());
                }
                return Err(err.into());
            }
        }
        Ok(ScanOutcome::Infected {
            scanner,
            scanner_version,
            signature_version,
            threat_name,
        }) => {
            diesel::update(document_files::table.filter(document_files::id.eq(document_file_id)))
                .set((
                    document_files::scan_status.eq(SCAN_STATUS_INFECTED),
                    document_files::scan_scanner.eq(Some(scanner)),
                    document_files::scan_scanner_version.eq(scanner_version),
                    document_files::scan_signature_version.eq(signature_version),
                    document_files::scan_threat_name.eq(threat_name),
                    document_files::scan_error.eq::<Option<String>>(None),
                    document_files::scan_completed_at.eq(Some(Utc::now())),
                    document_files::updated_at.eq(Utc::now()),
                ))
                .execute(&mut db)
                .await?;
        }
        Err(err) if state.malware_scanner.failure_policy() == MalwareScannerFailurePolicy::Open => {
            diesel::update(document_files::table.filter(document_files::id.eq(document_file_id)))
                .set((
                    document_files::scan_status.eq(SCAN_STATUS_NOT_REQUIRED),
                    document_files::scan_scanner.eq(Some(err.scanner)),
                    document_files::scan_error.eq(Some(err.reason)),
                    document_files::scan_completed_at.eq(Some(Utc::now())),
                    document_files::updated_at.eq(Utc::now()),
                ))
                .execute(&mut db)
                .await?;

            let mut fast_jobs = state.fast_jobs.as_ref().clone();
            let mut medium_jobs = state.medium_jobs.as_ref().clone();
            if enqueue_post_upload_processing_jobs(
                &mut fast_jobs,
                &mut medium_jobs,
                document_file_id,
            )
            .await
            .is_err()
            {
                mark_scan_error(
                    &mut db,
                    document_file_id,
                    "autofile",
                    "Failed to enqueue document processing jobs",
                )
                .await?;
                return Err(
                    std::io::Error::other("Failed to enqueue document processing jobs").into(),
                );
            }
        }
        Err(err) => {
            diesel::update(document_files::table.filter(document_files::id.eq(document_file_id)))
                .set((
                    document_files::scan_status.eq(SCAN_STATUS_ERROR),
                    document_files::scan_scanner.eq(Some(err.scanner)),
                    document_files::scan_error.eq(Some(err.reason)),
                    document_files::scan_completed_at.eq(Some(Utc::now())),
                    document_files::updated_at.eq(Utc::now()),
                ))
                .execute(&mut db)
                .await?;
        }
    }

    Ok(())
}

async fn mark_scan_error(
    db: &mut AsyncPgConnection,
    document_file_id: i64,
    scanner: &str,
    reason: &str,
) -> JobResult<()> {
    diesel::update(document_files::table.filter(document_files::id.eq(document_file_id)))
        .set((
            document_files::scan_status.eq(SCAN_STATUS_ERROR),
            document_files::scan_scanner.eq(Some(scanner.to_string())),
            document_files::scan_error.eq(Some(reason.to_string())),
            document_files::scan_completed_at.eq(Some(Utc::now())),
            document_files::updated_at.eq(Utc::now()),
        ))
        .execute(db)
        .await?;
    Ok(())
}

#[derive(Clone, Debug)]
struct ProcessCommandOutput {
    success: bool,
    status: String,
    stdout: String,
    stderr: String,
}

/// Maximum characters kept per captured stream when reporting a failed
/// process. External tools log to files rather than stdio, so failures are
/// short; this bound keeps error payloads (and retried-job payloads) sane.
const MAX_PROCESS_STREAM_CHARS: usize = 4000;

pub(crate) fn truncate_process_stream(stream: &str) -> String {
    let trimmed = stream.trim();
    if trimmed.chars().count() <= MAX_PROCESS_STREAM_CHARS {
        return trimmed.to_string();
    }
    let truncated: String = trimmed.chars().take(MAX_PROCESS_STREAM_CHARS).collect();
    format!("{truncated}… (truncated)")
}

#[async_trait::async_trait]
trait ProcessRunner {
    async fn run(&self, program: &str, args: &[String]) -> std::io::Result<ProcessCommandOutput>;
}

struct TokioProcessRunner;

#[async_trait::async_trait]
impl ProcessRunner for TokioProcessRunner {
    async fn run(&self, program: &str, args: &[String]) -> std::io::Result<ProcessCommandOutput> {
        let output = sanitized_command(program).args(args).output().await?;

        Ok(ProcessCommandOutput {
            success: output.status.success(),
            status: output.status.to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
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
        // NB: several tools (notably `soffice`) report conversion progress and
        // errors on stdout rather than stderr, so both streams are included.
        let error = std::io::Error::other(format!(
            "{tool_name} failed with status {}: stdout: {} stderr: {}",
            output.status,
            truncate_process_stream(&output.stdout),
            truncate_process_stream(&output.stderr),
        ));
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
) -> JobResult<()> {
    process_file_pages_inner(document_file_id, state).await
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
    let mut document_file = match document_files::table
        .find(document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await
    {
        Ok(document_file) => document_file,
        Err(diesel::result::Error::NotFound) => {
            tracing::info!(
                document_file_id,
                "document file no longer exists; skipping page processing"
            );
            return Ok(());
        }
        Err(err) => return Err(err.into()),
    };
    if let Some(message) = document_file.content_availability().unavailable_message() {
        tracing::info!(
            document_file_id,
            message,
            "skipping page processing for unavailable file"
        );
        return Ok(());
    }
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
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Csv => {
                process_file_pages_csv(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Tsv => {
                process_file_pages_tsv(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::OfficeDocument => {
                process_file_pages_office_document(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Html => {
                process_file_pages_html(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Markdown => {
                process_file_pages_markdown(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Pdf => {
                process_file_pages_pdf(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
            DocumentFileContentType::Image => {
                process_file_pages_image(
                    document_file_id,
                    &document_file,
                    &temp_dir,
                    &temp_file,
                    state.clone(),
                )
                .await
            }
        }
    }
    .await;

    if let Err(err) = tokio::fs::remove_dir_all(&temp_dir).await {
        tracing::warn!(error = %err, path = %temp_file, "failed to remove temp dir");
    }

    result?;

    enqueue_classify_document_if_needed(&document_file, state).await?;

    Ok(())
}

async fn enqueue_classify_document_if_needed(
    document_file: &DocumentFile,
    state: Data<Arc<AppState>>,
) -> JobResult<()> {
    let mut db = state.db_pool.get().await?;
    let document_type_id = documents::table
        .find(document_file.document_id)
        .select(documents::document_type_id)
        .first::<i64>(&mut db)
        .await?;

    if !should_classify_processed_document(document_type_id) {
        return Ok(());
    }
    drop(db);

    let mut medium_jobs = state.medium_jobs.as_ref().clone();
    medium_jobs
        .push(MediumJob::ClassifyDocument {
            document_id: document_file.document_id,
            user_id: SYSTEM_USER_ID,
        })
        .await?;

    Ok(())
}

fn should_classify_processed_document(document_type_id: i64) -> bool {
    document_type_id == UNSPECIFIED_DOCUMENT_TYPE_ID
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
        PANDOC_PDF_ENGINE.to_string(),
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
        PANDOC_PDF_ENGINE.to_string(),
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

    // Each conversion gets its own LibreOffice user profile. Concurrent
    // `soffice` processes (e.g. the pages job and the thumbnail job for the
    // same upload) otherwise share `~/.config/libreoffice` and can fail with
    // an bare exit status 1 while fighting over the profile lock — with no
    // output on either stream. Nesting the profile inside `convert_dir` keeps
    // the existing cleanup below covering it.
    let profile_dir = convert_dir.join("lo-profile");
    tokio::fs::create_dir_all(&profile_dir).await?;

    let conversion_result = async {
        let soffice_input = if copied_input {
            office_input.as_path()
        } else {
            source_path
        };

        let args = vec![
            format!(
                "-env:UserInstallation=file://{}",
                profile_dir.to_string_lossy()
            ),
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

    if copied_input && let Err(err) = tokio::fs::remove_file(&office_input).await {
        tracing::warn!(error = %err, path = %office_input.display(), "failed to remove soffice temp input");
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
        PANDOC_PDF_ENGINE.to_string(),
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
        PANDOC_PDF_ENGINE.to_string(),
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

    let affected = diesel::update(document_files::table.find(document_file_id))
        .set(document_files::pages.eq(pages as i32))
        .execute(&mut db)
        .await?;
    if affected == 0 {
        return cancel_document_file_processing(document_file_id, None, "page processing");
    }

    for page in 1..=pages {
        if !document_file_exists(&mut db, document_file_id).await? {
            return cancel_document_file_processing(
                document_file_id,
                Some(page),
                "page processing",
            );
        }

        {
            tracing::info!(document_file_id, page, "extracting text for page");
            let text =
                extract_pdf_page_text(temp_file.to_owned(), page, temp_dir, state.clone()).await?;
            if !upsert_document_file_page(&mut db, document_file_id, page as i32, Some(text))
                .await?
            {
                return cancel_document_file_processing(
                    document_file_id,
                    Some(page),
                    "page processing",
                );
            }
        }

        {
            if !document_file_exists(&mut db, document_file_id).await? {
                return cancel_document_file_processing(
                    document_file_id,
                    Some(page),
                    "page processing",
                );
            }

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
            let ocr_text = extract_page_ocr(&image_path, temp_dir, state.clone()).await?;
            if !upsert_document_file_ocr_page(
                &mut db,
                document_file_id,
                page as i32,
                Some(ocr_text),
            )
            .await?
            {
                return cancel_document_file_processing(
                    document_file_id,
                    Some(page),
                    "page processing",
                );
            }
            remove_temp_file_best_effort(&image_path, "page image").await;
        }
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

    let affected = diesel::update(document_files::table.find(document_file_id))
        .set(document_files::pages.eq(1))
        .execute(&mut db)
        .await?;
    if affected == 0 {
        return cancel_document_file_processing(document_file_id, Some(1), "image page processing");
    }

    if !upsert_document_file_page(&mut db, document_file_id, 1, None).await? {
        return cancel_document_file_processing(document_file_id, Some(1), "image page processing");
    }

    if !document_file_exists(&mut db, document_file_id).await? {
        return cancel_document_file_processing(document_file_id, Some(1), "image page processing");
    }

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
    let ocr_text = extract_page_ocr(&image_path, temp_dir, state.clone()).await?;
    if !upsert_document_file_ocr_page(&mut db, document_file_id, 1, Some(ocr_text)).await? {
        return cancel_document_file_processing(document_file_id, Some(1), "image page processing");
    }
    remove_temp_file_best_effort(&image_path, "image page").await;

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

pub(crate) async fn document_file_exists(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
) -> JobResult<bool> {
    let exists = select(diesel::dsl::exists(
        document_files::table.filter(document_files::id.eq(document_file_id)),
    ))
    .get_result::<bool>(db)
    .await?;

    Ok(exists)
}

fn cancel_document_file_processing(
    document_file_id: i64,
    page: Option<u32>,
    processing_name: &'static str,
) -> JobResult<()> {
    if let Some(page) = page {
        tracing::info!(
            document_file_id,
            page,
            "document file no longer exists; cancelling {processing_name}"
        );
    } else {
        tracing::info!(
            document_file_id,
            "document file no longer exists; cancelling {processing_name}"
        );
    }

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
) -> JobResult<bool> {
    let result = diesel::insert_into(document_file_pages::table)
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
        .await;

    match result {
        Ok(_) => Ok(true),
        Err(err) if is_foreign_key_violation(&err) => Ok(false),
        Err(err) => Err(err.into()),
    }
}

async fn upsert_document_file_ocr_page(
    db: &mut diesel_async::AsyncPgConnection,
    document_file_id: i64,
    page_number: i32,
    ocr_content: Option<String>,
) -> JobResult<bool> {
    let result = diesel::insert_into(document_file_ocr_pages::table)
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
        .await;

    match result {
        Ok(_) => Ok(true),
        Err(err) if is_foreign_key_violation(&err) => Ok(false),
        Err(err) => Err(err.into()),
    }
}

fn is_foreign_key_violation(err: &DieselError) -> bool {
    matches!(
        err,
        DieselError::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _)
    )
}

/**
 * Internal function to extract the text content of a specific page in a PDF document
 * by running `pdftotext` on the file and capturing the output.
 */
async fn extract_pdf_page_text(
    file: String,
    page: u32,
    temp_dir: &Path,
    _state: Data<Arc<AppState>>,
) -> JobResult<String> {
    let runner = TokioProcessRunner;
    extract_pdf_page_text_with_runner(file, page, temp_dir, &runner).await
}

async fn extract_pdf_page_text_with_runner<R: ProcessRunner + ?Sized>(
    file: String,
    page: u32,
    temp_dir: &Path,
    runner: &R,
) -> JobResult<String> {
    let output_path = temp_dir.join(format!("page-{page}-text.txt"));
    let args = vec![
        "-f".to_string(),
        page.to_string(),
        "-l".to_string(),
        page.to_string(),
        file,
        output_path.to_string_lossy().to_string(),
    ];
    run_process(runner, "pdftotext", &args, "pdftotext").await?;

    let text = read_string_and_remove_file(&output_path).await?;
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
    let output = sanitized_command("pdftocairo")
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
        .output()
        .await?;
    if !output.status.success() {
        let error = std::io::Error::other(format!(
            "pdftocairo failed with status {}: {}",
            output.status,
            truncate_process_stream(&String::from_utf8_lossy(&output.stderr)),
        ));
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
    let output = sanitized_command("magick")
        .arg(&input_path)
        .arg("-auto-orient")
        .arg("-strip")
        .arg(output_path)
        .output()
        .await?;

    if !output.status.success() {
        let error = std::io::Error::other(format!(
            "magick failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
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
async fn extract_page_ocr(
    image_path: &Path,
    temp_dir: &Path,
    _state: Data<Arc<AppState>>,
) -> JobResult<String> {
    let runner = TokioProcessRunner;
    extract_page_ocr_with_runner(image_path, temp_dir, &runner).await
}

async fn extract_page_ocr_with_runner<R: ProcessRunner + ?Sized>(
    image_path: &Path,
    temp_dir: &Path,
    runner: &R,
) -> JobResult<String> {
    let output_base = temp_dir.join(format!("ocr-{}", Uuid::new_v4()));
    let output_path = output_base.with_extension("txt");
    let args = vec![
        image_path.to_string_lossy().to_string(),
        output_base.to_string_lossy().to_string(),
    ];
    run_process(runner, "tesseract", &args, "tesseract").await?;

    let text = read_string_and_remove_file(&output_path).await?;
    Ok(text)
}

async fn read_string_and_remove_file(path: &Path) -> JobResult<String> {
    let text = tokio::fs::read_to_string(path).await?;
    tokio::fs::remove_file(path).await?;
    Ok(text)
}

async fn remove_temp_file_best_effort(path: &Path, artifact_name: &str) {
    if let Err(err) = tokio::fs::remove_file(path).await {
        tracing::warn!(error = %err, path = %path.display(), artifact_name, "failed to remove temp artifact");
    }
}

/**
 * Internal function to count the number of pages in a PDF document
 * by running `pdfinfo` on the file and parsing the output.
 */
async fn count_pages(file: String, _state: Data<Arc<AppState>>) -> JobResult<u32> {
    // 2) run `pdfinfo input.pdf` and parse the output to get the page count
    let output = sanitized_command("pdfinfo").arg(file).output().await?;

    if !output.status.success() {
        let error = std::io::Error::other(format!(
            "pdfinfo failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
        return Err(error.into());
    }

    let output_str = String::from_utf8(output.stdout)?;
    for line in output_str.lines() {
        if line.starts_with("Pages:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() == 2
                && let Ok(page_count) = parts[1].parse::<u32>()
            {
                return Ok(page_count);
            }
        }
    }

    Err(std::io::Error::other("Failed to parse page count from pdfinfo output").into())
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
    let tmp_dir = std::env::temp_dir().join(format!("{}-{}", tempfile_prefix, Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp_dir).await?;
    let tmp_file = tmp_dir.join("staged-file");
    write_stream_to_file(object.body.into_async_read(), &tmp_file).await?;

    Ok((tmp_dir, tmp_file.to_string_lossy().to_string()))
}

async fn write_stream_to_file<R>(mut reader: R, path: &Path) -> JobResult<()>
where
    R: AsyncRead + Unpin,
{
    let mut file = tokio::fs::File::create(path).await?;
    tokio::io::copy(&mut reader, &mut file).await?;
    file.sync_all().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    #[derive(Clone)]
    enum FakeRunnerMode {
        SuccessCreatesOutput,
        SuccessCreatesTextOutput(String),
        SuccessNoOutput,
        Failure {
            status: String,
            stdout: String,
            stderr: String,
        },
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
                        stdout: String::new(),
                        stderr: String::new(),
                    })
                }
                FakeRunnerMode::SuccessCreatesTextOutput(text) => {
                    if program == "pdftotext" {
                        let output_path = args.last().expect("output path not passed");
                        tokio::fs::write(output_path, text).await?;
                    } else if program == "tesseract" {
                        let output_base = args.get(1).expect("output base not passed");
                        tokio::fs::write(format!("{output_base}.txt"), text).await?;
                    }

                    Ok(ProcessCommandOutput {
                        success: true,
                        status: "0".to_string(),
                        stdout: String::new(),
                        stderr: String::new(),
                    })
                }
                FakeRunnerMode::SuccessNoOutput => Ok(ProcessCommandOutput {
                    success: true,
                    status: "0".to_string(),
                    stdout: String::new(),
                    stderr: String::new(),
                }),
                FakeRunnerMode::Failure {
                    status,
                    stdout,
                    stderr,
                } => Ok(ProcessCommandOutput {
                    success: false,
                    status: status.clone(),
                    stdout: stdout.clone(),
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
    fn should_classify_processed_document_only_for_unclassified_type() {
        assert!(should_classify_processed_document(1));
        assert!(!should_classify_processed_document(2));
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
                "--pdf-engine=weasyprint",
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
                "--pdf-engine=weasyprint",
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
                "--pdf-engine=weasyprint",
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
                "--pdf-engine=weasyprint",
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
            stdout: String::new(),
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
            stdout: String::new(),
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
        assert!(
            args[0].starts_with("-env:UserInstallation=file://"),
            "first arg should isolate the LibreOffice profile, got {}",
            args[0]
        );
        assert_eq!(args[1], "--headless");
        assert_eq!(args[2], "--convert-to");
        assert_eq!(args[3], "pdf");
        assert_eq!(args[4], "--outdir");
        assert!(args[5].contains("soffice-out-"));
        assert!(args[6].ends_with("invoice.docx"));

        let temp_outdir = Path::new(&args[5]);
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
            stdout: "Error: source file could not be loaded".to_string(),
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
        assert!(err_text.contains("Error: source file could not be loaded"));

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

    #[tokio::test]
    async fn write_stream_to_file_writes_stream_without_collecting_first() {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");
        let output_path = test_dir.join("streamed-output");
        let contents = b"first chunk\nsecond chunk\nthird chunk";

        write_stream_to_file(&contents[..], &output_path)
            .await
            .expect("stream write should succeed");

        let written = tokio::fs::read(&output_path)
            .await
            .expect("failed to read streamed output");
        assert_eq!(written, contents);

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn pdf_text_extraction_writes_to_temp_file_not_stdout() {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessCreatesTextOutput(
            "extracted pdf text".to_string(),
        ));

        let text =
            extract_pdf_page_text_with_runner("/tmp/source.pdf".to_string(), 7, &test_dir, &runner)
                .await
                .expect("pdf text extraction should succeed");

        assert_eq!(text, "extracted pdf text");
        assert_single_process_call(
            &runner,
            "pdftotext",
            &[
                "-f",
                "7",
                "-l",
                "7",
                "/tmp/source.pdf",
                test_dir.join("page-7-text.txt").to_string_lossy().as_ref(),
            ],
        );
        assert!(
            !tokio::fs::try_exists(test_dir.join("page-7-text.txt"))
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn ocr_extraction_writes_to_temp_file_not_stdout() {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");
        let image_path = test_dir.join("page-1.png");
        tokio::fs::write(&image_path, b"png")
            .await
            .expect("failed to write page image");
        let runner = FakeProcessRunner::new(FakeRunnerMode::SuccessCreatesTextOutput(
            "ocr text".to_string(),
        ));

        let text = extract_page_ocr_with_runner(&image_path, &test_dir, &runner)
            .await
            .expect("ocr extraction should succeed");

        assert_eq!(text, "ocr text");
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        let (program, args) = &calls[0];
        assert_eq!(program, "tesseract");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], image_path.to_string_lossy());
        assert!(args[1].starts_with(test_dir.to_string_lossy().as_ref()));
        assert!(args[1].contains("ocr-"));
        assert!(!args.iter().any(|arg| arg == "stdout"));
        assert!(
            !tokio::fs::try_exists(format!("{}.txt", args[1]))
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn read_string_and_remove_file_returns_contents_and_deletes_file() {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");
        let output_path = test_dir.join("text-output.txt");
        tokio::fs::write(&output_path, "extracted text")
            .await
            .expect("failed to write text output");

        let text = read_string_and_remove_file(&output_path)
            .await
            .expect("read and cleanup should succeed");

        assert_eq!(text, "extracted text");
        assert!(
            !tokio::fs::try_exists(&output_path)
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn remove_temp_file_best_effort_deletes_existing_file() {
        let test_dir = std::env::temp_dir().join(format!("autofile-test-{}", Uuid::new_v4()));
        tokio::fs::create_dir_all(&test_dir)
            .await
            .expect("failed to create test dir");
        let output_path = test_dir.join("page-1.png");
        tokio::fs::write(&output_path, b"png")
            .await
            .expect("failed to write page image");

        remove_temp_file_best_effort(&output_path, "page image").await;

        assert!(
            !tokio::fs::try_exists(&output_path)
                .await
                .expect("try_exists failed")
        );

        tokio::fs::remove_dir_all(&test_dir)
            .await
            .expect("failed to clean test dir");
    }

    #[tokio::test]
    async fn remove_temp_file_best_effort_ignores_missing_file() {
        let missing_path =
            std::env::temp_dir().join(format!("autofile-test-missing-{}", Uuid::new_v4()));

        remove_temp_file_best_effort(&missing_path, "missing temp file").await;

        assert!(
            !tokio::fs::try_exists(&missing_path)
                .await
                .expect("try_exists failed")
        );
    }
}
