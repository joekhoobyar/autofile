use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::shared::errors::ApiError;

pub struct TempUpload {
    pub path: std::path::PathBuf,
    pub size: i64,
    pub checksum_sha256: String,
}

/**
 * Write the contents of a multipart field to a temporary file,
 * returning a `TempUpload` struct containing the file path, size, and SHA-256 checksum.
 */
pub async fn write_field_to_temp_file(
    field: &mut axum::extract::multipart::Field<'_>,
) -> Result<TempUpload, ApiError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("autofile-upload-{}", Uuid::new_v4()));
    let mut temp_file = tokio::fs::File::create(&temp_path).await.map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to create temp file: {}", e))
    })?;

    let mut size: i64 = 0;
    let mut hasher = Sha256::new();
    loop {
        let chunk = field
            .chunk()
            .await
            .map_err(|e| ApiError::bad_request(&format!("Failed to read file data: {}", e)))?;

        let Some(chunk) = chunk else {
            break;
        };

        size += chunk.len() as i64;
        hasher.update(&chunk);
        temp_file.write_all(&chunk).await.map_err(|e| {
            ApiError::internal_server_error(&format!("Failed to buffer upload: {}", e))
        })?;
    }

    temp_file.flush().await.map_err(|e| {
        ApiError::internal_server_error(&format!("Failed to finalize temp file: {}", e))
    })?;

    Ok(TempUpload {
        path: temp_path,
        size,
        checksum_sha256: format!("{:x}", hasher.finalize()),
    })
}
