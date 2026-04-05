use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{Delete, ObjectIdentifier};

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
    let mut request = client.put_object().bucket(bucket).key(s3_key).body(body);

    if let Some(ct) = content_type {
        request = request.content_type(ct);
    }

    request.send().await.map_err(|e| {
        // Log detailed error for debugging
        eprintln!("S3 upload error details:");
        eprintln!("  Bucket: {}", bucket);
        eprintln!("  Key: {}", s3_key);
        eprintln!("  Error: {:?}", e);

        // Return detailed error message
        S3Error(format!(
            "Failed to upload to S3 bucket '{}' key '{}': {}",
            bucket, s3_key, e
        ))
    })?;

    Ok(())
}

pub async fn delete_from_s3(client: &S3Client, bucket: &str, key: &str) -> Result<(), S3Error> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| {
            eprintln!(
                "Failed to delete S3 object '{}' from bucket '{}': {:?}",
                key, bucket, e
            );
            S3Error(format!("Failed to delete from S3: {:?}", e))
        })?;

    Ok(())
}

pub async fn delete_prefix_from_s3(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: &str,
) -> Result<(), aws_sdk_s3::Error> {
    let mut continuation = None;

    loop {
        let resp = client
            .list_objects_v2()
            .bucket(bucket)
            .prefix(prefix)
            .set_continuation_token(continuation)
            .send()
            .await?;

        let objects: Vec<ObjectIdentifier> = resp
            .contents()
            .iter()
            .filter_map(|o| o.key())
            .map(|k| ObjectIdentifier::builder().key(k).build())
            .collect::<Result<Vec<_>, _>>()?;

        if !objects.is_empty() {
            let delete = Delete::builder().set_objects(Some(objects)).build()?;

            client
                .delete_objects()
                .bucket(bucket)
                .delete(delete)
                .send()
                .await?;
        }

        if !resp.is_truncated().unwrap_or(false) {
            break;
        }

        continuation = resp.next_continuation_token().map(ToOwned::to_owned);
    }

    Ok(())
}
