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
use tokio::signal;
use tokio::sync::watch;
use tokio::time::{Duration, sleep};

use autofile_api::infrastructure::queue::{build_monitor, create_storages};
use autofile_api::run_migrations;
use autofile_api::shared::app_state::AppState;
use autofile_api::shared::errors::ApiError;
use autofile_api::shared::extractors::DbConn;
use autofile_api::shared::openapi::build_openapi_router;

#[tokio::main]
async fn main() {
    // Dump the OpenAPI specification without requiring any runtime
    // configuration (JWT, Redis, database, S3). Release workflows reuse the
    // built image for this (overriding the tini entrypoint):
    // `docker run --rm --entrypoint /usr/local/bin/autofile-api <image> --dump-openapi`.
    if std::env::args().any(|arg| arg == "--dump-openapi") {
        let (_, openapi) = build_openapi_router();
        let json = openapi
            .to_pretty_json()
            .expect("OpenAPI spec should serialize to JSON");
        println!("{json}");
        return;
    }

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Initialize JSON Web Token
    jsonwebtoken::crypto::aws_lc::DEFAULT_PROVIDER
        .install_default()
        .expect("Failed to install rustls crypto provider");
    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET not set")
        .into_bytes();

    // Redis storage (queue) for background jobs.
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379/?connect_timeout=2&timeout=2".to_string());
    let storages = create_storages(&redis_url).await;

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = create_db_pool_with_retry(&database_url).await;

    run_migrations_with_retry(&database_url).await;

    // Initialize S3 client
    let s3_endpoint = std::env::var("AWS_ENDPOINT_URL_S3").ok();
    let mut s3_defaults = aws_config::defaults(aws_config::BehaviorVersion::latest());
    if let Some(endpoint) = s3_endpoint.as_deref() {
        s3_defaults = s3_defaults.endpoint_url(endpoint);
    }
    let s3_config = s3_defaults.load().await;
    let mut s3_client_config = aws_sdk_s3::Config::builder()
        .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
        .credentials_provider(s3_config.credentials_provider().unwrap())
        .region(s3_config.region().cloned());
    if let Some(endpoint) = s3_endpoint.as_deref() {
        s3_client_config = s3_client_config
            .endpoint_url(endpoint)
            .force_path_style(true); // Required for local S3-compatible endpoints.
    }
    let s3_client = aws_sdk_s3::Client::from_conf(s3_client_config.build());
    let s3_bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET must be set");

    // Initialize allowed origins for CORS
    let allowed_origins = std::env::var("ALLOWED_ORIGINS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|origin| !origin.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|origins| !origins.is_empty())
        .unwrap_or_else(|| vec!["http://localhost:5173".to_string()]);

    // Build shared application state
    let app_state = Arc::new(AppState {
        db_pool,
        s3_client: Arc::new(s3_client),
        s3_bucket: Arc::new(s3_bucket),
        jwt_secret: Arc::new(jwt_secret),
        fast_jobs: Arc::new(storages.fast),
        medium_jobs: Arc::new(storages.medium),
        slow_jobs: Arc::new(storages.slow),
    });

    // Configure background worker monitor and shutdown signal.
    let worker_monitor = build_monitor(app_state.clone());
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    tokio::spawn(async move {
        shutdown_signal().await;
        let _ = shutdown_tx.send(true);
    });
    let worker_shutdown_rx = shutdown_rx.clone();
    let worker_handle = tokio::spawn(async move {
        let mut shutdown_rx = worker_shutdown_rx;

        worker_monitor
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
                .filter_map(|origin| match origin.parse::<HeaderValue>() {
                    Ok(value) => Some(value),
                    Err(err) => {
                        tracing::warn!(origin = %origin, error = %err, "invalid origin in ALLOWED_ORIGINS");
                        None
                    }
                })
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

    let (api_v1_router, openapi) = build_openapi_router();

    // Build the router (wrap state in Arc for efficient sharing)
    let app = Router::new()
        .route("/api/v1/health/ready", get(health_ready))
        .nest("/api/v1", api_v1_router)
        .merge(utoipa_swagger_ui::SwaggerUi::new("/api/docs").url("/api/v1/openapi.json", openapi))
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

async fn create_db_pool_with_retry(database_url: &str) -> bb8::Pool<AsyncPgConnection> {
    let mut attempt = 1;

    loop {
        let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

        match bb8::Pool::builder()
            .connection_timeout(Duration::from_secs(5))
            .build(config)
            .await
        {
            Ok(pool) => {
                tracing::info!(attempts = attempt, "Database connection pool created");
                return pool;
            }
            Err(err) => {
                tracing::warn!(
                    attempt,
                    error = %err,
                    "Database connection pool unavailable; retrying in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                attempt += 1;
            }
        }
    }
}

async fn run_migrations_with_retry(database_url: &str) {
    let mut attempt = 1;

    loop {
        match run_migrations(database_url).await {
            Ok(()) => return,
            Err(err) => {
                tracing::warn!(
                    attempt,
                    error = %err,
                    "Database migrations failed; retrying in 5 seconds"
                );
                sleep(Duration::from_secs(5)).await;
                attempt += 1;
            }
        }
    }
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
