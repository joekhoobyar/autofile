use aws_sdk_s3::error::SdkError;
use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::Response,
};
use chrono::{DateTime, Utc};
use httpdate::{fmt_http_date, parse_http_date};
use tokio_util::io::ReaderStream;

use crate::AppState;
use crate::shared::util::ApiError;

pub async fn serve_s3_image(
    state: &AppState,
    headers: &HeaderMap,
    s3_key: &str,
    fallback_last_modified: Option<DateTime<Utc>>,
    missing_message: &str,
) -> Result<Response, ApiError> {
    let object = state
        .s3_client
        .get_object()
        .bucket(state.s3_bucket.as_str())
        .key(s3_key)
        .send()
        .await
        .map_err(|e| match e {
            SdkError::ServiceError(service_error) if service_error.err().is_no_such_key() => {
                ApiError::not_found(missing_message)
            }
            _ => ApiError::internal_server_error(&format!("S3 download failed: {e}")),
        })?;

    let last_modified = fallback_last_modified
        .map(std::time::SystemTime::from)
        .or_else(|| {
            object
                .last_modified()
                .copied()
                .and_then(|value| std::time::SystemTime::try_from(value).ok())
        });

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
    let body = Body::from_stream(ReaderStream::new(object.body.into_async_read()));
    let mut response = Response::new(body);
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("image/png"),
    );
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
