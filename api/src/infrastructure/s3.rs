use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::types::{CompletedMultipartUpload, CompletedPart, Delete, ObjectIdentifier};
use std::path::Path;
use tokio::io::AsyncReadExt;

const MAX_SINGLE_PUT_SIZE: i64 = 16 * 1024 * 1024;
const MULTIPART_UPLOAD_PART_SIZE: usize = 8 * 1024 * 1024;

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

pub async fn upload_file_to_s3(
    client: &S3Client,
    bucket: &str,
    s3_key: &str,
    path: &Path,
    size: i64,
    content_type: Option<&str>,
) -> Result<(), S3Error> {
    if size <= MAX_SINGLE_PUT_SIZE {
        let body = ByteStream::from_path(path)
            .await
            .map_err(|e| S3Error(format!("Failed to read file for S3 upload: {e}")))?;
        return upload_to_s3(client, bucket, s3_key, body, content_type).await;
    }

    upload_file_to_s3_multipart(client, bucket, s3_key, path, content_type).await
}

async fn upload_file_to_s3_multipart(
    client: &S3Client,
    bucket: &str,
    s3_key: &str,
    path: &Path,
    content_type: Option<&str>,
) -> Result<(), S3Error> {
    let mut create_request = client.create_multipart_upload().bucket(bucket).key(s3_key);
    if let Some(ct) = content_type {
        create_request = create_request.content_type(ct);
    }

    let create_output = create_request.send().await.map_err(|e| {
        eprintln!("S3 multipart upload create error details:");
        eprintln!("  Bucket: {}", bucket);
        eprintln!("  Key: {}", s3_key);
        eprintln!("  Error: {:?}", e);
        S3Error(format!(
            "Failed to create S3 multipart upload for bucket '{}' key '{}': {}",
            bucket, s3_key, e
        ))
    })?;
    let upload_id = create_output
        .upload_id()
        .ok_or_else(|| S3Error("S3 multipart upload did not return an upload_id".to_string()))?;

    let upload_result = async {
        let mut file = tokio::fs::File::open(path)
            .await
            .map_err(|e| S3Error(format!("Failed to open file for S3 multipart upload: {e}")))?;
        let mut parts = Vec::new();
        let mut part_number = 1;

        loop {
            let mut buffer = Vec::with_capacity(MULTIPART_UPLOAD_PART_SIZE);
            while buffer.len() < MULTIPART_UPLOAD_PART_SIZE {
                let remaining = MULTIPART_UPLOAD_PART_SIZE - buffer.len();
                let mut chunk = vec![0; remaining];
                let bytes_read = file.read(&mut chunk).await.map_err(|e| {
                    S3Error(format!("Failed to read S3 multipart upload part: {e}"))
                })?;
                if bytes_read == 0 {
                    break;
                }
                buffer.extend_from_slice(&chunk[..bytes_read]);
            }

            if buffer.is_empty() {
                break;
            }

            let part_output = client
                .upload_part()
                .bucket(bucket)
                .key(s3_key)
                .upload_id(upload_id)
                .part_number(part_number)
                .body(ByteStream::from(buffer))
                .send()
                .await
                .map_err(|e| {
                    eprintln!("S3 multipart upload part error details:");
                    eprintln!("  Bucket: {}", bucket);
                    eprintln!("  Key: {}", s3_key);
                    eprintln!("  Part: {}", part_number);
                    eprintln!("  Error: {:?}", e);
                    S3Error(format!(
                        "Failed to upload S3 multipart part {} for bucket '{}' key '{}': {}",
                        part_number, bucket, s3_key, e
                    ))
                })?;
            let e_tag = part_output.e_tag().ok_or_else(|| {
                S3Error(format!(
                    "S3 multipart upload part {} did not return an ETag",
                    part_number
                ))
            })?;
            parts.push(
                CompletedPart::builder()
                    .part_number(part_number)
                    .e_tag(e_tag)
                    .build(),
            );
            part_number += 1;
        }

        let completed_upload = CompletedMultipartUpload::builder()
            .set_parts(Some(parts))
            .build();

        client
            .complete_multipart_upload()
            .bucket(bucket)
            .key(s3_key)
            .upload_id(upload_id)
            .multipart_upload(completed_upload)
            .send()
            .await
            .map_err(|e| {
                eprintln!("S3 multipart upload complete error details:");
                eprintln!("  Bucket: {}", bucket);
                eprintln!("  Key: {}", s3_key);
                eprintln!("  Error: {:?}", e);
                S3Error(format!(
                    "Failed to complete S3 multipart upload for bucket '{}' key '{}': {}",
                    bucket, s3_key, e
                ))
            })?;

        Ok(())
    }
    .await;

    if upload_result.is_err() {
        let _ = client
            .abort_multipart_upload()
            .bucket(bucket)
            .key(s3_key)
            .upload_id(upload_id)
            .send()
            .await;
    }

    upload_result
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
