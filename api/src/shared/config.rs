/// Maximum upload size configuration.
///
/// A single `MAX_UPLOAD_SIZE_MB` environment variable controls both upload
/// endpoints (`POST /api/v1/documents` and
/// `POST /api/v1/documents/{document_id}/files`). The value is read once at
/// startup; invalid values fail fast so a misconfigured limit never boots
/// silently.
pub const DEFAULT_MAX_UPLOAD_SIZE_MB: u64 = 100;

/// Headroom added to Axum's `DefaultBodyLimit` layer on top of the configured
/// file limit. Multipart framing (boundaries, text fields) counts toward the
/// total body size, so the layer is deliberately looser; the precise per-file
/// limit is enforced while streaming in `crate::shared::uploads`.
pub const MAX_UPLOAD_MULTIPART_OVERHEAD_BYTES: usize = 2 * 1024 * 1024;

const BYTES_PER_MB: u64 = 1024 * 1024;

/// Parse an optional raw `MAX_UPLOAD_SIZE_MB` value into bytes.
///
/// - `None` (unset) or blank → default (100 MB).
/// - Otherwise the value must be a positive integer number of megabytes.
/// - Zero, negative, non-numeric, or overflowing values return an error
///   describing the problem; callers fail fast at startup.
pub fn parse_max_upload_bytes(raw: Option<&str>) -> Result<usize, String> {
    let raw = raw.map(str::trim).filter(|s| !s.is_empty());
    let Some(raw) = raw else {
        return Ok(mb_to_bytes(DEFAULT_MAX_UPLOAD_SIZE_MB));
    };

    let mb: u64 = raw.parse().map_err(|_| {
        format!("MAX_UPLOAD_SIZE_MB must be a positive integer (megabytes), got {raw:?}")
    })?;
    if mb == 0 {
        return Err(format!(
            "MAX_UPLOAD_SIZE_MB must be a positive integer (megabytes), got {raw:?}"
        ));
    }

    let bytes = mb
        .checked_mul(BYTES_PER_MB)
        .and_then(|b| usize::try_from(b).ok())
        .ok_or_else(|| {
            format!("MAX_UPLOAD_SIZE_MB value {raw:?} is too large to represent in bytes")
        })?;
    Ok(bytes)
}

/// Convert whole megabytes to bytes, saturating on overflow.
pub fn mb_to_bytes(mb: u64) -> usize {
    mb.checked_mul(BYTES_PER_MB)
        .and_then(|b| usize::try_from(b).ok())
        .unwrap_or(usize::MAX)
}

/// Read `MAX_UPLOAD_SIZE_MB` from the environment, failing fast on invalid
/// values. Unset/blank falls back to the default.
pub fn max_upload_bytes_from_env() -> usize {
    match parse_max_upload_bytes(std::env::var("MAX_UPLOAD_SIZE_MB").ok().as_deref()) {
        Ok(bytes) => bytes,
        Err(err) => panic!("{err}"),
    }
}

/// Format a byte limit as whole megabytes for user-facing error messages.
pub fn bytes_to_mb(bytes: u64) -> u64 {
    bytes / BYTES_PER_MB
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_100mb_when_unset_or_blank() {
        assert_eq!(
            parse_max_upload_bytes(None).expect("default should parse"),
            100 * 1024 * 1024
        );
        assert_eq!(
            parse_max_upload_bytes(Some("")).expect("blank should parse"),
            100 * 1024 * 1024
        );
        assert_eq!(
            parse_max_upload_bytes(Some("   ")).expect("whitespace should parse"),
            100 * 1024 * 1024
        );
    }

    #[test]
    fn parses_valid_megabyte_values() {
        assert_eq!(
            parse_max_upload_bytes(Some("1")).expect("1 MB should parse"),
            1024 * 1024
        );
        assert_eq!(
            parse_max_upload_bytes(Some(" 250 ")).expect("padded value should parse"),
            250 * 1024 * 1024
        );
    }

    #[test]
    fn rejects_invalid_values() {
        for raw in ["0", "-5", "abc", "1.5", "100MB"] {
            assert!(
                parse_max_upload_bytes(Some(raw)).is_err(),
                "{raw:?} should be rejected"
            );
        }
    }
}
