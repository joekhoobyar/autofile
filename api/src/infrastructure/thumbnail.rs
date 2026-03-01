use std::sync::Arc;

use aws_sdk_s3::primitives::ByteStream;
use serde::{Deserialize, Serialize};

use apalis::prelude::*;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tokio::process::Command;
use uuid::Uuid;

use crate::domain::document_files::DocumentFile;
use crate::infrastructure::s3::upload_to_s3;
use crate::schema::document_files;
use crate::shared::app_state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateThumbnail {
    pub document_file_id: i64,
    pub page: u32,
    pub width: u32,
}

pub async fn generate_thumbnail(
    job: GenerateThumbnail,
    state: Data<Arc<AppState>>,
) -> Result<(), Error> {
    tracing::info!(?job, "generating thumbnail");

    // Load the document file from the database to get the S3 location.
    let mut db = state
        .db_pool
        .get()
        .await
        .map_err(to_job_error)?;
    let document_file = document_files::table
        .find(job.document_file_id)
        .select(DocumentFile::as_select())
        .first::<DocumentFile>(&mut db)
        .await
        .map_err(to_job_error)?;

    // 1) download doc from object storage via job.object_key
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
    let tmp_dir = std::env::temp_dir().join(format!("autofile-thumb-{}", Uuid::new_v4()));
    tokio::fs::create_dir_all(&tmp_dir)
        .await
        .map_err(to_job_error)?;
    let input_path = tmp_dir.join("input.pdf");
    tokio::fs::write(&input_path, file_bytes)
        .await
        .map_err(to_job_error)?;

    // 2) generate thumbnail
    //    pdftocairo -png -singlefile -f 1 -l 1 -scale-to-x 300 -scale-to-y -1 input.pdf _thumb
    //
    let output_prefix = tmp_dir.join("_thumb");
    let status = Command::new("pdftocairo")
        .arg("-png")
        .arg("-singlefile")
        .arg("-f")
        .arg(job.page.to_string())
        .arg("-l")
        .arg(job.page.to_string())
        .arg("-scale-to-x")
        .arg(job.width.to_string())
        .arg("-scale-to-y")
        .arg("-1")
        .arg(&input_path)
        .arg(&output_prefix)
        .status()
        .await
        .map_err(to_job_error)?;
    if !status.success() {
        let error = std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("pdftocairo failed with status {status}"),
        );
        return Err(to_job_error(error));
    }

    // 3) upload thumbnail + update DB
    //    upload_to_s3(..., ..., ByteStream::from_path("_thumb.png"), ...)
    //
    let thumb_path = tmp_dir.join("_thumb.png");
    let thumb_key = format!("{}/_thumb.png", document_file.s3_prefix);
    let body = ByteStream::from_path(&thumb_path)
        .await
        .map_err(to_job_error)?;
    upload_to_s3(
        &state.s3_client,
        state.s3_bucket.as_str(),
        &thumb_key,
        body,
        Some("image/png"),
    )
    .await
    .map_err(to_job_error)?;

    let _ = tokio::fs::remove_dir_all(&tmp_dir).await;

    Ok(())
}

fn to_job_error<E>(err: E) -> Error
where
    E: std::error::Error + Send + Sync + 'static,
{
    let boxed: BoxDynError = Box::new(err);
    Error::Failed(Arc::new(boxed))
}
