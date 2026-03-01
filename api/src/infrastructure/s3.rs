use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;

#[derive(Debug)]
pub struct S3Error(String);

impl std::fmt::Display for S3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S3 Error: {}", self.0)
    }
}

impl std::error::Error for S3Error {}

pub async fn upload_to_s3(
    client: &S3Client,
    bucket: &str,
    s3_key: &str,
    body: ByteStream,
    content_type: Option<&str>,
) -> Result<(), S3Error> {
    let mut request = client
        .put_object()
        .bucket(bucket)
        .key(s3_key)
        .body(body);

    if let Some(ct) = content_type {
        request = request.content_type(ct);
    }

    request
        .send()
        .await
        .map_err(|e| {
            // Log detailed error for debugging
            eprintln!("S3 upload error details:");
            eprintln!("  Bucket: {}", bucket);
            eprintln!("  Key: {}", s3_key);
            eprintln!("  Error: {:?}", e);

            // Return detailed error message
            S3Error(format!("Failed to upload to S3 bucket '{}' key '{}': {}", bucket, s3_key, e))
        })?;

    Ok(())
}

pub async fn delete_from_s3(
    client: &S3Client,
    bucket: &str,
    s3_key: &str,
) -> Result<(), S3Error> {
    client
        .delete_object()
        .bucket(bucket)
        .key(s3_key)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Failed to delete S3 object '{}' from bucket '{}': {:?}", s3_key, bucket, e);
            S3Error(format!("Failed to delete from S3: {:?}", e))
        })?;

    Ok(())
}
