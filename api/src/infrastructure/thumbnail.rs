use serde::{Serialize, Deserialize};

use apalis::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateThumbnail {
    pub document_file_id: i64,
    pub page: u32,
    pub width: u32,
}

pub async fn generate_thumbnail(job: GenerateThumbnail) -> Result<(), Error> {
    // 1) download doc from object storage via job.object_key
    // 2) generate thumbnail (often external tool; consider spawn_blocking / tokio::process)
    // 3) upload thumbnail + update DB

    tracing::info!(?job, "generating thumbnail");
    Ok(())
}
