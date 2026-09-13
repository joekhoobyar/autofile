use std::sync::atomic::{AtomicU64, Ordering};

use autofile_api::run_migrations;
use diesel::sql_query;
use diesel_async::AsyncConnection;
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, bb8};
use testcontainers::ContainerAsync;
use testcontainers::ImageExt;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::postgres::Postgres;
use tokio::sync::OnceCell;

const TEMPLATE_DB_NAME: &str = "autofile_test_template";
const MAINTENANCE_DB_NAME: &str = "postgres";

/// One Postgres container per test binary, shared by all tests in the binary
/// via `cargo test`'s in-process threads. Each test still gets an isolated
/// database cloned from a migrated template, so fixture IDs can overlap
/// across tests without interference.
static SHARED_SERVER: OnceCell<SharedTestServer> = OnceCell::const_new();
static DB_COUNTER: AtomicU64 = AtomicU64::new(0);

struct SharedTestServer {
    _container: ContainerAsync<Postgres>,
    base_url: String,
    template_db_url: String,
}

pub struct TestDatabase {
    pub pool: bb8::Pool<AsyncPgConnection>,
}

impl TestDatabase {
    pub async fn new() -> Self {
        let shared = SHARED_SERVER
            .get_or_init(|| async {
                let container = Postgres::default()
                    .with_tag("17-alpine")
                    .start()
                    .await
                    .expect("postgres should start");
                let port = container
                    .get_host_port_ipv4(5432)
                    .await
                    .expect("postgres port should be mapped");
                let base_url = format!("postgres://postgres:postgres@127.0.0.1:{port}");
                let template_db_url = format!("{base_url}/{TEMPLATE_DB_NAME}");

                // Create the template database with a one-shot maintenance
                // connection, then migrate it once. Per-test databases clone
                // it below, so migrations never run per test.
                let maintenance_url = format!("{base_url}/{MAINTENANCE_DB_NAME}");
                let mut maintenance_conn = AsyncPgConnection::establish(&maintenance_url)
                    .await
                    .expect("maintenance connection should succeed");
                sql_query(format!("CREATE DATABASE \"{TEMPLATE_DB_NAME}\""))
                    .execute(&mut maintenance_conn)
                    .await
                    .expect("template database should be created");

                run_migrations(&template_db_url)
                    .await
                    .expect("migrations should run");

                SharedTestServer {
                    _container: container,
                    base_url,
                    template_db_url,
                }
            })
            .await;

        // Clone the migrated template: milliseconds, versus seconds for a
        // fresh container plus a full migration run. Each clone uses a fresh
        // one-shot maintenance connection (plus retries) rather than a pooled
        // one, so a burst of parallel clones can't hit a stale pooled
        // connection dropped by the container's port forwarding.
        let db_name = format!(
            "test_db_{}_{}",
            std::process::id(),
            DB_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let maintenance_url = format!("{}/{MAINTENANCE_DB_NAME}", shared.base_url);
        let mut last_err = String::new();
        let mut cloned = false;
        for _ in 0..3 {
            let clone_result = async {
                let mut admin_conn = AsyncPgConnection::establish(&maintenance_url)
                    .await
                    .map_err(|e| format!("maintenance connection failed: {e}"))?;
                sql_query(format!(
                    "CREATE DATABASE \"{db_name}\" TEMPLATE \"{TEMPLATE_DB_NAME}\""
                ))
                .execute(&mut admin_conn)
                .await
                .map_err(|e| format!("clone failed: {e}"))?;
                Ok::<(), String>(())
            }
            .await;
            match clone_result {
                Ok(()) => {
                    cloned = true;
                    break;
                }
                Err(e) => last_err = e,
            }
        }
        assert!(cloned, "test database should be cloned: {last_err}");

        let database_url = shared.template_db_url.replace(TEMPLATE_DB_NAME, &db_name);
        // Bound per-test connections: most tests hold a single pooled
        // connection, while the index-rebuild tests nest two pool checkouts
        // (rebuild holds one while each per-document update takes another)
        // on top of the test-held connection, for a peak of three. Capping
        // at that depth avoids nested-checkout deadlocks while keeping the
        // worst case (cap × parallel tests) under Postgres's default
        // max_connections=100 on many-core runners.
        let pool = bb8::Pool::builder()
            .max_size(3)
            .build(AsyncDieselConnectionManager::<AsyncPgConnection>::new(
                database_url,
            ))
            .await
            .expect("pool should build");

        Self { pool }
    }
}
