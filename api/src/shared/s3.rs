use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use chrono::{DateTime, Utc};
use httpdate::{fmt_http_date, parse_http_date};
use tokio::time::{Duration, sleep};
use tokio_util::io::ReaderStream;

use crate::shared::app_state::AppState;
use crate::shared::util::ApiError;

const S3_GET_MAX_ATTEMPTS: u8 = 3;
type GetObjectSdkError = SdkError<GetObjectError>;

pub async fn serve_s3_file(
    state: &AppState,
    headers: &HeaderMap,
    s3_key: &str,
    fallback_last_modified: Option<DateTime<Utc>>,
    missing_message: &str,
    fallback_content_type: Option<&str>,
) -> Result<Response, ApiError> {
    let mut attempt = 1_u8;
    let object = loop {
        match state
            .s3_client
            .get_object()
            .bucket(state.s3_bucket.as_str())
            .key(s3_key)
            .send()
            .await
        {
            Ok(object) => break Ok(object),
            Err(err) if attempt < S3_GET_MAX_ATTEMPTS && is_retryable_s3_get_error(&err) => {
                let retry_in_ms = 100_u64 * (1_u64 << (attempt - 1));
                let status = s3_get_error_status_code(&err);
                tracing::warn!(
                    attempt,
                    max_attempts = S3_GET_MAX_ATTEMPTS,
                    bucket = %state.s3_bucket,
                    key = s3_key,
                    status,
                    error = %err,
                    retry_in_ms,
                    "S3 get_object failed with retryable error; retrying"
                );
                sleep(Duration::from_millis(retry_in_ms)).await;
                attempt += 1;
            }
            Err(err) => break Err(err),
        }
    }
    .map_err(|e| match e {
        SdkError::ServiceError(service_error) if service_error.err().is_no_such_key() => {
            ApiError::not_found(missing_message)
        }
        _ => ApiError::internal_server_error(&format!("S3 download failed: {e}")),
    })?;

    let last_modified = object
        .last_modified()
        .copied()
        .and_then(|value| std::time::SystemTime::try_from(value).ok())
        .or_else(|| fallback_last_modified.map(std::time::SystemTime::from));

    if let (Some(last_modified), Some(if_modified_since)) = (
        last_modified,
        headers
            .get(header::IF_MODIFIED_SINCE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| parse_http_date(value).ok()),
    ) {
        if last_modified <= if_modified_since {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_MODIFIED;
            let headers = response.headers_mut();
            let last_modified = fmt_http_date(last_modified);
            if let Ok(value) = header::HeaderValue::from_str(&last_modified) {
                headers.insert(header::LAST_MODIFIED, value);
            }
            headers.insert(
                header::CACHE_CONTROL,
                header::HeaderValue::from_static("public, must-revalidate"),
            );
            return Ok(response);
        }
    }

    let content_length = object.content_length();
    let content_type = object
        .content_type()
        .map(str::to_owned)
        .or_else(|| fallback_content_type.map(str::to_owned));
    let body = Body::from_stream(ReaderStream::new(object.body.into_async_read()));
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    if let Some(content_type) = content_type {
        if let Ok(value) = header::HeaderValue::from_str(&content_type) {
            headers.insert(header::CONTENT_TYPE, value);
        }
    }
    headers.insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("public, must-revalidate"),
    );
    if let Some(last_modified) = last_modified {
        let last_modified = fmt_http_date(last_modified);
        if let Ok(value) = header::HeaderValue::from_str(&last_modified) {
            headers.insert(header::LAST_MODIFIED, value);
        }
    }
    if let Some(content_length) = content_length {
        if content_length > 0 {
            if let Ok(value) = header::HeaderValue::from_str(&content_length.to_string()) {
                headers.insert(header::CONTENT_LENGTH, value);
            }
        }
    }

    Ok(response)
}

fn is_retryable_s3_get_error(err: &GetObjectSdkError) -> bool {
    match err {
        SdkError::DispatchFailure(_) | SdkError::TimeoutError(_) => true,
        SdkError::ServiceError(service_error) => {
            matches!(
                service_error.raw().status().as_u16(),
                429 | 500 | 502 | 503 | 504
            )
        }
        _ => false,
    }
}

fn s3_get_error_status_code(err: &GetObjectSdkError) -> Option<u16> {
    match err {
        SdkError::ServiceError(service_error) => Some(service_error.raw().status().as_u16()),
        _ => None,
    }
}
