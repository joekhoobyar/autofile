#[macro_use] extern crate rocket;

use std::sync::OnceLock;
use rocket::http::{Status, Method};
use rocket::serde::json::Json;
use rocket::figment::Figment;

use rocket_cors::{AllowedOrigins, CorsOptions};

use rocket_db_pools::{Connection, Database};
use rocket_db_pools::diesel::{PgPool, prelude::*};

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub struct OurAllowedOrigins(pub Vec<String>);

#[derive(Database)]
#[database("autofile")]
pub struct Db(PgPool);

fn database_url(figment: &Figment, db_name: &str) -> Option<String> {
    figment
        .extract_inner::<String>(&format!("databases.{db_name}.url"))
        .ok()
}

mod auth;
mod schema;
mod resource {
    pub mod auth;
    pub mod cabinets;
    pub mod document_types_metadata_types;
    pub mod document_types;
    pub mod documents;
    pub mod metadata_types;
    pub mod users;
}
mod util;
mod s3;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[launch]
async fn rocket() -> _ {

    // Initialize S3 client
    let s3_endpoint = std::env::var("AWS_ENDPOINT_URL_S3").expect("AWS_ENDPOINT_URL_S3 not set");
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
    let s3_bucket = std::env::var("S3_BUCKET").expect("S3_BUCKET not set");

    let allowed_origins = vec![
        "http://localhost:5173".to_string(),
    ];

    let cors = CorsOptions::default()
        .allowed_origins(AllowedOrigins::some_exact(&allowed_origins))
        .allowed_methods(
            vec![Method::Get, Method::Post, Method::Put, Method::Patch, Method::Delete, Method::Options]
                .into_iter()
                .map(From::from)
                .collect(),
        )
        .allow_credentials(true);

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");

    rocket::build()
        .manage(auth::JwtSecret(secret.into_bytes()))
        .manage(OurAllowedOrigins(allowed_origins))
        .manage(s3_client)
        .manage(s3_bucket)
        .attach(Db::init())
        .attach(cors.to_cors().unwrap())
        .attach(rocket::fairing::AdHoc::try_on_ignite("Diesel Migrations", run_migrations))
        .mount("/", routes![index, health_ready])
        .attach(resource::auth::stage())
        .attach(resource::cabinets::stage())
        .attach(resource::document_types_metadata_types::stage())
        .attach(resource::document_types::stage())
        .attach(resource::documents::stage())
        .attach(resource::metadata_types::stage())
        .attach(resource::users::stage())
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/health/ready")]
async fn health_ready(mut db: Connection<Db>) -> Result<Json<ReadyResponse>, Status> {
    // Minimal readiness check: can we run a trivial query?
    // SELECT 1
    let one: i32 = diesel::select(diesel::dsl::sql::<diesel::sql_types::Integer>("1"))
        .get_result(&mut db)
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    let db_ok = one == 1;

    Ok(Json(ReadyResponse { ok: db_ok, db: db_ok }))
}

#[derive(serde::Serialize)]
pub struct ReadyResponse {
    ok: bool,
    db: bool,
}

async fn run_migrations(rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
    let db_url = match database_url(rocket.figment(), "autofile") {
        Some(u) => u,
        None => return Err(rocket),
    };

    let res = tokio::task::spawn_blocking(move || {
        let mut conn = <diesel::PgConnection as diesel::Connection>::establish(&db_url)
            .expect("pg connect failed");

        conn.run_pending_migrations(MIGRATIONS)
            .expect("migrations failed");

        Ok::<(), ()>(())
    })
    .await;

    match res {
        Ok(Ok(())) => Ok(rocket),
        _ => Err(rocket),
    }
}

static IS_PRODUCTION: OnceLock<bool> = OnceLock::new();

pub fn is_production() -> bool {
    *IS_PRODUCTION.get_or_init(|| {
        std::env::var("APP_MODE")
            .map(|v| v.eq_ignore_ascii_case("production"))
            .unwrap_or(false)
    })
}