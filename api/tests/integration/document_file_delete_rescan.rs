//! Opt-in coverage for task 21: unsafe files can still be deleted and
//! submitted for explicit rescan/retry.
//!
//! Unlike the metadata-gate tests, these paths need a full `AppState`:
//! deletion removes the S3 object prefix afterwards, and rescan enqueues a
//! `ScanDocumentFile` job. That requires S3-compatible storage (MinIO) and a
//! live Valkey server (only to construct the queue storages held by
//! `AppState`; neither path needs a scanner daemon because rescan only
//! resets the row to `pending` and enqueues the job).

use std::sync::Arc;
use std::time::Duration;

use apalis_redis::RedisStorage;
use autofile_api::AppState;
use autofile_api::application::document_files::{
    delete_document_file, mark_document_file_scan_not_required, rescan_document_file,
};
use autofile_api::application::jobs::{FastJob, MediumJob, SlowJob};
use autofile_api::application::malware_scanning::FakeMalwareScanner;
use autofile_api::domain::document_files::{
    SCAN_STATUS_INFECTED, SCAN_STATUS_NOT_REQUIRED, SCAN_STATUS_PENDING,
};
use autofile_api::schema::{app_settings, document_files};
use autofile_api::shared::config::{
    MalwareScannerConfig, MalwareScannerFailurePolicy, MalwareScannerProvider,
};
use aws_sdk_s3::primitives::ByteStream;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use testcontainers::core::IntoContainerPort;
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};
use testcontainers_modules::valkey::{VALKEY_PORT, Valkey};

use crate::support::db::TestDatabase;
use crate::support::fixtures::{
    insert_document, insert_document_file, insert_document_type, insert_user,
};

const USER_ID: i64 = 8001;
const DOCUMENT_TYPE_ID: i64 = 8002;
const DOCUMENT_ID: i64 = 8003;
const UNSAFE_FILE_ID: i64 = 8004;
const SAFE_FILE_ID: i64 = 8005;
const BUCKET: &str = "autofile-test-documents";

struct TestEnv {
    state: Arc<AppState>,
    db: TestDatabase,
    s3: aws_sdk_s3::Client,
    unsafe_prefix: String,
    safe_prefix: String,
}

async fn seed_two_files(
    db: &mut diesel_async::pooled_connection::bb8::PooledConnection<
        '_,
        diesel_async::AsyncPgConnection,
    >,
    unsafe_status: &str,
) -> (String, String) {
    insert_user(db, USER_ID, "delete-user", "delete-user@example.com").await;
    diesel::update(app_settings::table)
        .set(app_settings::virus_scanning_enabled.eq(true))
        .execute(&mut *db)
        .await
        .expect("virus scanning should be enabled for rescan");
    insert_document_type(db, DOCUMENT_TYPE_ID, "delete-type", "Delete Type", USER_ID).await;
    insert_document(
        db,
        DOCUMENT_ID,
        "Delete Document",
        DOCUMENT_TYPE_ID,
        USER_ID,
    )
    .await;
    insert_document_file(db, UNSAFE_FILE_ID, DOCUMENT_ID, "unsafe.pdf", USER_ID).await;
    insert_document_file(db, SAFE_FILE_ID, DOCUMENT_ID, "safe.pdf", USER_ID).await;

    let unsafe_prefix = format!("unsafe-prefix-{UNSAFE_FILE_ID}");
    let safe_prefix = format!("safe-prefix-{SAFE_FILE_ID}");
    diesel::update(document_files::table.filter(document_files::id.eq(UNSAFE_FILE_ID)))
        .set((
            document_files::s3_prefix.eq(&unsafe_prefix),
            document_files::scan_status.eq(unsafe_status),
            document_files::scan_requested.eq(true),
        ))
        .execute(&mut *db)
        .await
        .expect("unsafe file update should succeed");
    diesel::update(document_files::table.filter(document_files::id.eq(SAFE_FILE_ID)))
        .set(document_files::s3_prefix.eq(&safe_prefix))
        .execute(&mut *db)
        .await
        .expect("safe file update should succeed");
    (unsafe_prefix, safe_prefix)
}

async fn setup_env(unsafe_status: &str) -> TestEnv {
    // RustFS (same image as docker-compose.yml) serves the S3-compatible API
    // on :9000. No container healthcheck exists, so bucket creation below
    // retries until the server is ready.
    let rustfs = GenericImage::new("rustfs/rustfs", "1.0.0-rc.6")
        .with_exposed_port(9000.tcp())
        .with_env_var("RUSTFS_ACCESS_KEY", "autofile-test-access")
        .with_env_var("RUSTFS_SECRET_KEY", "autofile-test-secret")
        .with_env_var("RUSTFS_ADDRESS", ":9000")
        .with_cmd(["/data"])
        .with_startup_timeout(Duration::from_secs(120))
        .start()
        .await
        .expect("RustFS container should start");
    let s3_port = rustfs
        .get_host_port_ipv4(9000)
        .await
        .expect("RustFS port should be mapped");

    let valkey = Valkey::default()
        .start()
        .await
        .expect("Valkey container should start");
    let valkey_port = valkey
        .get_host_port_ipv4(VALKEY_PORT)
        .await
        .expect("Valkey port should be mapped");

    let s3 = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .endpoint_url(format!("http://127.0.0.1:{s3_port}"))
            .credentials_provider(aws_sdk_s3::config::Credentials::new(
                "autofile-test-access",
                "autofile-test-secret",
                None,
                None,
                "test",
            ))
            .region(aws_sdk_s3::config::Region::new("us-east-1"))
            .force_path_style(true)
            .build(),
    );
    let mut bucket_ready = false;
    for _ in 0..60 {
        match s3.create_bucket().bucket(BUCKET).send().await {
            Ok(_) => {
                bucket_ready = true;
                break;
            }
            Err(_) => tokio::time::sleep(Duration::from_secs(2)).await,
        }
    }
    assert!(bucket_ready, "test bucket should be created");

    let redis_conn = apalis_redis::connect(format!("redis://127.0.0.1:{valkey_port}"))
        .await
        .expect("Valkey connection should succeed");

    let db = TestDatabase::new().await;
    let mut conn = db.pool.get().await.expect("db connection should succeed");
    let (unsafe_prefix, safe_prefix) = seed_two_files(&mut conn, unsafe_status).await;
    drop(conn);

    for (prefix, filename) in [
        (unsafe_prefix.clone(), "unsafe.pdf"),
        (safe_prefix.clone(), "safe.pdf"),
    ] {
        s3.put_object()
            .bucket(BUCKET)
            .key(format!("{prefix}/{filename}"))
            .body(ByteStream::from_static(b"file bytes"))
            .send()
            .await
            .expect("test object should upload");
    }

    let state = Arc::new(AppState {
        db_pool: db.pool.clone(),
        s3_client: Arc::new(s3.clone()),
        s3_bucket: Arc::new(BUCKET.to_string()),
        jwt_secret: Arc::new(b"test-jwt-secret".to_vec()),
        fast_jobs: Arc::new(RedisStorage::<FastJob>::new(redis_conn.clone())),
        medium_jobs: Arc::new(RedisStorage::<MediumJob>::new(redis_conn.clone())),
        slow_jobs: Arc::new(RedisStorage::<SlowJob>::new(redis_conn)),
        malware_scanner_config: MalwareScannerConfig {
            provider: MalwareScannerProvider::Disabled,
            clamav_host: "localhost".to_string(),
            clamav_port: 3310,
            clamav_connect_timeout: Duration::from_secs(5),
            clamav_scan_timeout: Duration::from_secs(120),
            failure_policy: MalwareScannerFailurePolicy::Closed,
        },
        malware_scanner: Arc::new(FakeMalwareScanner::new([])),
        max_upload_bytes: 100 * 1024 * 1024,
    });

    // Keep containers alive for the whole test.
    std::mem::forget(rustfs);
    std::mem::forget(valkey);

    TestEnv {
        state,
        db,
        s3,
        unsafe_prefix,
        safe_prefix,
    }
}

async fn object_keys(s3: &aws_sdk_s3::Client, prefix: &str) -> Vec<String> {
    s3.list_objects_v2()
        .bucket(BUCKET)
        .prefix(format!("{prefix}/"))
        .send()
        .await
        .expect("object listing should succeed")
        .contents
        .unwrap_or_default()
        .into_iter()
        .filter_map(|object| object.key)
        .collect()
}

#[tokio::test]
#[ignore = "requires Docker for Postgres, MinIO, and Valkey containers"]
async fn pending_file_can_be_deleted() {
    let env = setup_env(SCAN_STATUS_PENDING).await;
    let mut db = env
        .db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    delete_document_file(env.state.clone(), &mut db, DOCUMENT_ID, UNSAFE_FILE_ID)
        .await
        .expect("pending file deletion should succeed");

    let remaining: Option<String> = document_files::table
        .filter(document_files::id.eq(UNSAFE_FILE_ID))
        .select(document_files::filename)
        .first(&mut db)
        .await
        .optional()
        .expect("lookup should succeed");
    assert!(remaining.is_none(), "unsafe file row should be gone");

    assert!(
        object_keys(&env.s3, &env.unsafe_prefix).await.is_empty(),
        "unsafe file objects should be removed from storage"
    );
    assert_eq!(
        object_keys(&env.s3, &env.safe_prefix).await,
        vec![format!("{}/safe.pdf", env.safe_prefix)],
        "sibling file object should be untouched"
    );

    let sibling: String = document_files::table
        .filter(document_files::id.eq(SAFE_FILE_ID))
        .select(document_files::filename)
        .first(&mut db)
        .await
        .expect("sibling file should remain");
    assert_eq!(sibling, "safe.pdf");
}

#[tokio::test]
#[ignore = "requires Docker for Postgres, MinIO, and Valkey containers"]
async fn infected_file_can_be_rescanned() {
    let env = setup_env(SCAN_STATUS_INFECTED).await;
    let mut db = env
        .db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let view = rescan_document_file(
        env.state.clone(),
        &mut db,
        USER_ID,
        DOCUMENT_ID,
        UNSAFE_FILE_ID,
    )
    .await
    .expect("infected file rescan should be accepted");
    assert_eq!(view.scan_status, SCAN_STATUS_PENDING);
    assert!(view.scan_requested);
    assert!(!view.content_available);

    let status: String = document_files::table
        .filter(document_files::id.eq(UNSAFE_FILE_ID))
        .select(document_files::scan_status)
        .first(&mut db)
        .await
        .expect("status lookup should succeed");
    assert_eq!(status, SCAN_STATUS_PENDING);

    let err = rescan_document_file(
        env.state.clone(),
        &mut db,
        USER_ID,
        DOCUMENT_ID,
        UNSAFE_FILE_ID,
    )
    .await
    .expect_err("resubmitting a pending file should be rejected");
    assert_eq!(err.status, axum::http::StatusCode::CONFLICT);
    assert_eq!(err.message, "File is already pending or actively scanning");
}

#[tokio::test]
#[ignore = "requires Docker for Postgres, MinIO, and Valkey containers"]
async fn admin_can_mark_pending_file_scan_not_required() {
    let env = setup_env(SCAN_STATUS_PENDING).await;
    let mut db = env
        .db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let view = mark_document_file_scan_not_required(
        env.state.clone(),
        &mut db,
        USER_ID,
        DOCUMENT_ID,
        UNSAFE_FILE_ID,
    )
    .await
    .expect("pending file should be marked scan not required");

    assert_eq!(view.scan_status, SCAN_STATUS_NOT_REQUIRED);
    assert!(!view.scan_requested);
    assert!(view.content_available);

    let row: (String, bool, Option<i64>) = document_files::table
        .filter(document_files::id.eq(UNSAFE_FILE_ID))
        .select((
            document_files::scan_status,
            document_files::scan_requested,
            document_files::scan_requested_by,
        ))
        .first(&mut db)
        .await
        .expect("file lookup should succeed");
    assert_eq!(row, (SCAN_STATUS_NOT_REQUIRED.to_string(), false, None));
}

#[tokio::test]
#[ignore = "requires Docker for Postgres, MinIO, and Valkey containers"]
async fn infected_file_cannot_be_marked_scan_not_required() {
    let env = setup_env(SCAN_STATUS_INFECTED).await;
    let mut db = env
        .db
        .pool
        .get()
        .await
        .expect("db connection should succeed");

    let err = mark_document_file_scan_not_required(
        env.state.clone(),
        &mut db,
        USER_ID,
        DOCUMENT_ID,
        UNSAFE_FILE_ID,
    )
    .await
    .expect_err("infected file should remain blocked");

    assert_eq!(err.status, axum::http::StatusCode::CONFLICT);
    assert_eq!(
        err.message,
        "Infected files cannot be marked as scan not required"
    );
}
