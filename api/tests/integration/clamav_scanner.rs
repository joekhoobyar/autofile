use std::time::Duration;

use autofile_api::application::malware_scanning::{
    ClamAvScanner, MalwareScanner, ScanMetadata, ScanOutcome,
};
use autofile_api::shared::config::{
    MalwareScannerConfig, MalwareScannerFailurePolicy, MalwareScannerProvider,
};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

#[tokio::test]
#[ignore = "requires Docker and pulls/runs the ClamAV container image"]
async fn clamav_scans_plain_text_file_as_clean() {
    let container = GenericImage::new("clamav/clamav", "stable")
        .with_exposed_port(3310.tcp())
        .with_wait_for(WaitFor::healthcheck())
        .with_startup_timeout(Duration::from_secs(300))
        .start()
        .await
        .expect("ClamAV container should start");
    let port = container
        .get_host_port_ipv4(3310)
        .await
        .expect("ClamAV port should be mapped");

    let scanner = ClamAvScanner::new(MalwareScannerConfig {
        provider: MalwareScannerProvider::ClamAv,
        clamav_host: "127.0.0.1".to_string(),
        clamav_port: port,
        clamav_connect_timeout: Duration::from_secs(5),
        clamav_scan_timeout: Duration::from_secs(120),
        failure_policy: MalwareScannerFailurePolicy::Closed,
    });
    let mut input = tokio::io::BufReader::new(&b"plain text file\n"[..]);

    let outcome = scanner
        .scan(
            &mut input,
            ScanMetadata {
                filename: Some("plain.txt".to_string()),
                content_type: Some("text/plain".to_string()),
                size_bytes: Some(16),
            },
        )
        .await
        .expect("plain text should scan successfully");

    assert!(
        matches!(&outcome, ScanOutcome::Clean { scanner, .. } if scanner == "clamav"),
        "expected clean ClamAV verdict, got {outcome:?}"
    );
}
