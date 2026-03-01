use std::sync::OnceLock;
use std::sync::Arc;

use axum::{
    routing::get,
    Router,
    Json,
    http::{Method, HeaderValue},
};
use tower::ServiceBuilder;
use tower_http::{
    trace::TraceLayer,
    cors::CorsLayer,
};
use tower_cookies::CookieManagerLayer;

use diesel_async::{
    pooled_connection::{bb8, AsyncDieselConnectionManager},
    AsyncPgConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

mod schema;
mod api {
    pub mod auth;
    pub mod cabinets;
    pub mod document_types;
    pub mod document_types_metadata_types;
    pub mod documents;
    pub mod metadata_types;
    pub mod users;
}
mod domain {
    pub mod cabinets;
    pub mod document_files;
    pub mod document_types;
    pub mod document_types_metadata_types;
    pub mod documents;
    pub mod metadata_types;
    pub mod users;
}
mod infrastructure {
    pub mod s3;
}
mod shared {
    pub mod app_state;
    pub mod auth;
    pub mod extractors;
    pub mod util;
}

use shared::extractors::DbConn;

use crate::shared::app_state::AppState;
use crate::shared::util::ApiError;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
        )
        .init();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Create bb8 connection pool for diesel-async
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(&database_url);
    let db_pool = bb8::Pool::builder()
        .build(config)
        .await
        .expect("Failed to create database connection pool");

    // Run migrations
    run_migrations(&database_url).await;

    // Initialize S3 client
    let s3_endpoint = std::env::var("AWS_ENDPOINT_URL_S3")
        .expect("AWS_ENDPOINT_URL_S3 not set");
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
            .force_path_style(true)  // Required for minio
            .build()
    );
    let s3_bucket = std::env::var("S3_BUCKET")
        .expect("S3_BUCKET not set");

    let allowed_origins = vec![
        "http://localhost:5173".to_string(),
    ];

    let jwt_secret = std::env::var("JWT_SECRET")
        .expect("JWT_SECRET not set")
        .into_bytes();

    // Build shared application state
    let app_state = AppState {
        db_pool,
        s3_client: Arc::new(s3_client),
        s3_bucket: Arc::new(s3_bucket),
        jwt_secret: Arc::new(jwt_secret),
    };

    // Configure CORS
    use axum::http::header;
    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .map(|origin| origin.parse::<HeaderValue>().unwrap())
                .collect::<Vec<_>>()
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
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ]);

    let api_v1= Router::new()
        .route("/health/ready", get(health_ready))
        .nest("/auth", api::auth::routes())
        .nest("/cabinets", api::cabinets::routes())
        .nest("/document-types", api::document_types::routes())
        .nest("/document-types-metadata-types", api::document_types_metadata_types::routes())
        .nest("/documents", api::documents::routes())
        .nest("/metadata-types", api::metadata_types::routes())
        .nest("/users", api::users::routes());

    // Build the router (wrap state in Arc for efficient sharing)
    let app = Router::new()
        .nest("/api/v1", api_v1)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CookieManagerLayer::new())
                .layer(cors)
        )
        .with_state(Arc::new(app_state));

    // Get bind address from environment or use default
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8000".to_string());

    tracing::info!("Starting server on {}", bind_addr);

    // Start the server
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server failed");
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

    Ok(Json(ReadyResponse { ok: db_ok, db: db_ok }))
}

#[derive(serde::Serialize)]
pub struct ReadyResponse {
    ok: bool,
    db: bool,
}

async fn run_migrations(database_url: &str) {
    let database_url = database_url.to_string();

    tokio::task::spawn_blocking(move || {
        let mut conn = <diesel::PgConnection as diesel::Connection>::establish(&database_url)
            .expect("Failed to establish connection for migrations");

        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        tracing::info!("Migrations completed successfully");
    })
    .await
    .expect("Migration task panicked");
}

static IS_PRODUCTION: OnceLock<bool> = OnceLock::new();

pub fn is_production() -> bool {
    *IS_PRODUCTION.get_or_init(|| {
        std::env::var("APP_MODE")
            .map(|v| v.eq_ignore_ascii_case("production"))
            .unwrap_or(false)
    })
}
