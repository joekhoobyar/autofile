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
const DEFAULT_CLAMAV_HOST: &str = "clamav";
const DEFAULT_CLAMAV_PORT: u16 = 3310;
const DEFAULT_CLAMAV_CONNECT_TIMEOUT_SECONDS: u64 = 5;
const DEFAULT_CLAMAV_SCAN_TIMEOUT_SECONDS: u64 = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalwareScannerProvider {
    Disabled,
    ClamAv,
    External,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MalwareScannerFailurePolicy {
    Closed,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MalwareScannerConfig {
    pub provider: MalwareScannerProvider,
    pub clamav_host: String,
    pub clamav_port: u16,
    pub clamav_connect_timeout: std::time::Duration,
    pub clamav_scan_timeout: std::time::Duration,
    pub failure_policy: MalwareScannerFailurePolicy,
}

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

pub fn malware_scanner_config_from_env() -> MalwareScannerConfig {
    match parse_malware_scanner_config(EnvVarSource) {
        Ok(config) => config,
        Err(err) => panic!("{err}"),
    }
}

pub trait VarSource {
    fn var(&self, key: &str) -> Option<String>;
}

struct EnvVarSource;

impl VarSource for EnvVarSource {
    fn var(&self, key: &str) -> Option<String> {
        std::env::var(key).ok()
    }
}

pub fn parse_malware_scanner_config(vars: impl VarSource) -> Result<MalwareScannerConfig, String> {
    let provider = match normalized_var(&vars, "MALWARE_SCANNER_PROVIDER").as_deref() {
        None | Some("disabled") => MalwareScannerProvider::Disabled,
        Some("clamav") => MalwareScannerProvider::ClamAv,
        Some("external") => MalwareScannerProvider::External,
        Some(value) => {
            return Err(format!(
                "MALWARE_SCANNER_PROVIDER must be one of disabled, clamav, or external, got {value:?}"
            ));
        }
    };

    let failure_policy = match normalized_var(&vars, "MALWARE_SCANNER_FAILURE_POLICY").as_deref() {
        None | Some("closed") => MalwareScannerFailurePolicy::Closed,
        Some("open") => MalwareScannerFailurePolicy::Open,
        Some(value) => {
            return Err(format!(
                "MALWARE_SCANNER_FAILURE_POLICY must be closed or open, got {value:?}"
            ));
        }
    };

    let clamav_host = vars
        .var("CLAMAV_HOST")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CLAMAV_HOST.to_string());
    let clamav_port = parse_u16_var(&vars, "CLAMAV_PORT", DEFAULT_CLAMAV_PORT)?;
    let clamav_connect_timeout = std::time::Duration::from_secs(parse_u64_var(
        &vars,
        "CLAMAV_CONNECT_TIMEOUT_SECONDS",
        DEFAULT_CLAMAV_CONNECT_TIMEOUT_SECONDS,
    )?);
    let clamav_scan_timeout = std::time::Duration::from_secs(parse_u64_var(
        &vars,
        "CLAMAV_SCAN_TIMEOUT_SECONDS",
        DEFAULT_CLAMAV_SCAN_TIMEOUT_SECONDS,
    )?);

    Ok(MalwareScannerConfig {
        provider,
        clamav_host,
        clamav_port,
        clamav_connect_timeout,
        clamav_scan_timeout,
        failure_policy,
    })
}

fn normalized_var(vars: &impl VarSource, key: &str) -> Option<String> {
    vars.var(key)
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

fn parse_u16_var(vars: &impl VarSource, key: &str, default: u16) -> Result<u16, String> {
    let Some(raw) = vars.var(key).map(|value| value.trim().to_string()) else {
        return Ok(default);
    };
    if raw.is_empty() {
        return Ok(default);
    }
    raw.parse::<u16>()
        .map_err(|_| format!("{key} must be a valid TCP port, got {raw:?}"))
        .and_then(|value| {
            if value == 0 {
                Err(format!("{key} must be a valid TCP port, got {raw:?}"))
            } else {
                Ok(value)
            }
        })
}

fn parse_u64_var(vars: &impl VarSource, key: &str, default: u64) -> Result<u64, String> {
    let Some(raw) = vars.var(key).map(|value| value.trim().to_string()) else {
        return Ok(default);
    };
    if raw.is_empty() {
        return Ok(default);
    }
    raw.parse::<u64>()
        .map_err(|_| format!("{key} must be a positive integer, got {raw:?}"))
        .and_then(|value| {
            if value == 0 {
                Err(format!("{key} must be a positive integer, got {raw:?}"))
            } else {
                Ok(value)
            }
        })
}

/// Format a byte limit as whole megabytes for user-facing error messages.
pub fn bytes_to_mb(bytes: u64) -> u64 {
    bytes / BYTES_PER_MB
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    struct TestVars(HashMap<&'static str, &'static str>);

    impl VarSource for TestVars {
        fn var(&self, key: &str) -> Option<String> {
            self.0.get(key).map(|value| value.to_string())
        }
    }

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

    #[test]
    fn malware_scanner_config_defaults_to_disabled_clamav_defaults() {
        let config = parse_malware_scanner_config(TestVars(HashMap::new())).unwrap();

        assert_eq!(config.provider, MalwareScannerProvider::Disabled);
        assert_eq!(config.clamav_host, "clamav");
        assert_eq!(config.clamav_port, 3310);
        assert_eq!(
            config.clamav_connect_timeout,
            std::time::Duration::from_secs(5)
        );
        assert_eq!(
            config.clamav_scan_timeout,
            std::time::Duration::from_secs(120)
        );
        assert_eq!(config.failure_policy, MalwareScannerFailurePolicy::Closed);
    }

    #[test]
    fn malware_scanner_config_parses_clamav_values() {
        let config = parse_malware_scanner_config(TestVars(HashMap::from([
            ("MALWARE_SCANNER_PROVIDER", "clamav"),
            ("CLAMAV_HOST", "scanner.local"),
            ("CLAMAV_PORT", "3311"),
            ("CLAMAV_CONNECT_TIMEOUT_SECONDS", "3"),
            ("CLAMAV_SCAN_TIMEOUT_SECONDS", "60"),
            ("MALWARE_SCANNER_FAILURE_POLICY", "open"),
        ])))
        .unwrap();

        assert_eq!(config.provider, MalwareScannerProvider::ClamAv);
        assert_eq!(config.clamav_host, "scanner.local");
        assert_eq!(config.clamav_port, 3311);
        assert_eq!(
            config.clamav_connect_timeout,
            std::time::Duration::from_secs(3)
        );
        assert_eq!(
            config.clamav_scan_timeout,
            std::time::Duration::from_secs(60)
        );
        assert_eq!(config.failure_policy, MalwareScannerFailurePolicy::Open);
    }

    #[test]
    fn malware_scanner_config_rejects_invalid_values() {
        for (key, value) in [
            ("MALWARE_SCANNER_PROVIDER", "unknown"),
            ("CLAMAV_PORT", "0"),
            ("CLAMAV_PORT", "abc"),
            ("CLAMAV_CONNECT_TIMEOUT_SECONDS", "0"),
            ("CLAMAV_SCAN_TIMEOUT_SECONDS", "abc"),
            ("MALWARE_SCANNER_FAILURE_POLICY", "maybe"),
        ] {
            assert!(
                parse_malware_scanner_config(TestVars(HashMap::from([(key, value)]))).is_err(),
                "{key}={value:?} should be rejected"
            );
        }
    }
}
