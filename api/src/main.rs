use std::sync::Arc;

use axum::{
    Json, Router,
    http::{HeaderValue, Method},
    routing::get,
};
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use diesel_async::{
    AsyncPgConnection,
    pooled_connection::{AsyncDieselConnectionManager, bb8},
};
use redis::AsyncCommands;
use tokio::signal;
use tokio::sync::watch;
use tokio::time::{Duration, timeout};

use apalis::layers::WorkerBuilderExt;
use apalis::layers::retry::RetryPolicy;
use apalis::prelude::*;
use apalis_redis::RedisStorage;

use autofile_api::application::jobs::{FastJob, MediumJob, handle_fast_job, handle_medium_job};
use autofile_api::run_migrations;
use autofile_api::shared::app_state::AppState;
use autofile_api::shared::extractors::DbConn;
use autofile_api::shared::util::ApiError;
use autofile_api::{
    api,
    application::jobs::{SlowJob, handle_slow_job},
};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Redis storage (queue) for background jobs.
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/?connect_timeout=2&timeout=2".to_string());
    check_redis(&redis_url).await.expect("Redis not reachable");
    let redis_conn = apalis_redis::connect(redis_url.clone())
        .await
        .expect("Could not connect to Redis");
    let fast_storage: RedisStorage<FastJob> = RedisStorage::new(redis_conn.clone());
    let medium_storage: RedisStorage<MediumJob> = RedisStorage::new(redis_conn.clone());
    let slow_storage: RedisStorage<SlowJob> = RedisStorage::new(redis_conn);

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Create bb8 connection pool for diesel-async
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&database_url);
    let db_pool = bb8::Pool::builder()
        .build(config)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations
    run_migrations(&database_url).await;

    // Initialize S3 client
    let s3_endpoint = std::env::var("AWS_ENDPOINT_URL_S3").expect("AWS_ENDPOINT_URL_S3 must be set");
    let s3_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .endpoint_url(&s3_endpoint)
        .load()
        .await;

    // Configure S3 client for minio (path-style addressing)
    let s3_client = aws_sdk_s3::Client::from_conf(
        aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .credentials_provider(s3_config.credentials_provider().unwrap())
            .region(s3_config.region().cloned())
            .endpoint_url(&s3_endpoint)
            .force_path_style(true) // Required for minio
            .build(),
    );
    let s3_bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");

    let allowed_origins = vec!["http://localhost:5173".to_string()];

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET not set")
        .into_bytes();

    // Build shared application state
    let app_state = Arc::new(AppState {
        db_pool,
        s3_client: Arc::new(s3_client),
        s3_bucket: Arc::new(s3_bucket),
        jwt_secret: Arc::new(jwt_secret),
        fast_jobs: Arc::new(fast_storage),
        medium_jobs: Arc::new(medium_storage),
        slow_jobs: Arc::new(slow_storage),
    });

    // Spawn apalis workers (in-process).
    let monitor = Monitor::new()
        .register({
            // One or more workers pulling from Redis
            WorkerBuilder::new("fast-job-worker")
                .retry(RetryPolicy::retries(7))
                .enable_tracing()
                .concurrency(4) // Adjust concurrency as needed
                .data(app_state.clone())
                .backend(app_state.fast_jobs.as_ref().clone())
                .build_fn(handle_fast_job)
        })
        .register({
            WorkerBuilder::new("medium-job-worker")
                .retry(RetryPolicy::retries(7))
                .enable_tracing()
                .concurrency(2)
                .data(app_state.clone())
                .backend(app_state.medium_jobs.as_ref().clone())
                .build_fn(handle_medium_job)
        })
        .register({
            WorkerBuilder::new("slow-job-worker")
                .retry(RetryPolicy::retries(7))
                .enable_tracing()
                .concurrency(2)
                .data(app_state.clone())
                .backend(app_state.slow_jobs.as_ref().clone())
                .build_fn(handle_slow_job)
        })
        .on_event(|e| tracing::info!("{e}"))
        // Wait 5 seconds after shutdown is triggered to allow any incomplete jobs to complete
        // .shutdown_timeout(Duration::from_secs(5))
        ;

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });

    let worker_shutdown_rx = shutdown_rx.clone();
    let worker_handle = tokio::spawn(async move {
        let mut shutdown_rx = worker_shutdown_rx;

        monitor
            .run_with_signal(async move {
                let _ = shutdown_rx.changed().await;
                Ok(())
            })
            .await
            .expect("Background worker failed");

        tracing::info!("Workers have been shut down");
    });

    // Configure CORS
    use axum::http::header;
    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .map(|origin| origin.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_credentials(true)
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT]);

    let api_v1 = Router::new()
        .route("/health/ready", get(health_ready))
        .nest("/auth", api::auth::routes())
        .nest("/cabinets", api::cabinets::routes())
        .nest("/cabinets", api::cabinet_documents::routes())
        .nest("/classifier-blocks", api::classifier_blocks::routes())
        .nest("/document-indexes", api::document_indexes::routes())
        .nest("/document-indexes", api::document_index_templates::routes())
        .nest("/document-indexes", api::document_index_values::routes())
        .nest("/document-types", api::document_types::routes())
        .nest(
            "/document-types-metadata-types",
            api::document_types_metadata_types::routes(),
        )
        .nest("/documents", api::documents::routes())
        .nest("/documents", api::document_file_pages::routes())
        .nest("/documents", api::document_files::routes())
        .nest("/documents", api::document_metadatas::routes())
        .nest("/metadata-types", api::metadata_types::routes())
        .nest("/users", api::users::routes())
        .nest("/tags", api::tags::routes())
        .nest("/tags", api::tag_documents::routes());

    // Build the router (wrap state in Arc for efficient sharing)
    let app = Router::new()
        .nest("/api/v1", api_v1)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CookieManagerLayer::new())
                .layer(cors),
        )
        .with_state(app_state.clone());

    // Get bind address from environment or use default
    let bind_addr = std::env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8000".to_string());

    tracing::info!("Starting server on {}", bind_addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind address");

    let mut shutdown_rx = shutdown_rx.clone();

    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.changed().await;
        })
        .await
        .expect("Server failed");

    tracing::info!("Server has been shut down");

    worker_handle.await.expect("Worker task panicked");
}

async fn check_redis(redis_url: &str) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let mut conn = timeout(
        Duration::from_secs(3),
        client.get_multiplexed_async_connection(),
    )
    .await??;

    timeout(Duration::from_secs(3), conn.ping::<String>()).await??;

    Ok(())
}

async fn health_ready(DbConn(mut conn): DbConn) -> Result<Json<ReadyResponse>, ApiError> {
    // Minimal readiness check: can we run a trivial query?
    // SELECT 1
    use diesel_async::RunQueryDsl;

    let one: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
        .get_result(&mut conn)
        .await
        .map_err(|e| ApiError::internal_server_error(&format!("Database query failed: {}", e)))?;

    let db_ok = one == 1;

    Ok(Json(ReadyResponse {
        ok: db_ok,
        db: db_ok,
    }))
}

#[derive(serde::Serialize)]
pub struct ReadyResponse {
    ok: bool,
    db: bool,
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Ctrl+C received, shutting down");
        },
        _ = terminate => {
            tracing::info!("Terminate signal received, shutting down");
        },
    }
}
