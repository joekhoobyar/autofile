//! Sanitized child-process construction.
//!
//! Document processing shells out to external tools (`pandoc`, `soffice`,
//! `weasyprint`, `pdftocairo`, `pdftotext`, `pdfinfo`, `magick`, `tesseract`)
//! with untrusted input files. `tokio::process::Command` inherits the full
//! parent environment by default, which would leak API secrets (`DATABASE_URL`,
//! `REDIS_URL`, `JWT_SECRET`, `AWS_*`, `S3_BUCKET`, ...) to those children.
//!
//! Always build child commands with [`sanitized_command`], which clears the
//! environment and re-applies only the safe allowlist (`PATH` and `HOME`,
//! taken from the main process). Everything else — including secret-bearing
//! vars (`DATABASE_URL`, `JWT_SECRET`, `AWS_*`, ...) — is dropped.

use tokio::process::Command;

/// Environment variables propagated from the main process to child commands.
/// Everything else is dropped.
const INHERITED_ENV_VARS: &[&str] = &["PATH", "HOME"];

/// Build a child command with a sanitized environment.
///
/// Clears every inherited variable (including secrets) via `env_clear()`,
/// then re-applies only [`INHERITED_ENV_VARS`] (`PATH` and `HOME`) with the
/// values from the main process. Callers add program arguments as usual.
pub fn sanitized_command(program: &str) -> Command {
    let mut command = Command::new(program);
    command.env_clear();
    for key in INHERITED_ENV_VARS {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::collections::HashSet;

    #[tokio::test]
    async fn child_env_contains_only_inherited_allowlist() {
        let output = sanitized_command("env")
            .output()
            .await
            .expect("`env` should run with sanitized environment");
        assert!(
            output.status.success(),
            "`env` failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("`env` output should be UTF-8");
        let vars: HashMap<&str, &str> = stdout
            .lines()
            .filter_map(|line| line.split_once('='))
            .collect();

        // The expected key set is exactly the allowlist entries present in the
        // parent. The test runner's own environment always carries additional
        // vars (e.g. `CARGO_*`), so requiring this exact set proves all other
        // inherited vars — including any secret-bearing names — are dropped
        // (`env_clear` is name-agnostic).
        let expected: HashSet<&str> = INHERITED_ENV_VARS
            .iter()
            .copied()
            .filter(|key| std::env::var_os(key).is_some())
            .collect();
        let keys: HashSet<&str> = vars.keys().copied().collect();
        assert_eq!(
            keys, expected,
            "child environment must contain only the inherited allowlist: {vars:?}"
        );
        for key in &expected {
            assert_eq!(
                vars.get(key),
                std::env::var_os(key)
                    .as_ref()
                    .and_then(|value| value.to_str())
                    .as_ref(),
                "child {key} must match the main process value"
            );
        }
    }
}
