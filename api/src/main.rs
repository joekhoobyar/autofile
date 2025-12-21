#[macro_use] extern crate rocket;

use rocket_db_pools::{mongodb, Database};
use rocket::serde::json::Json;
use mongodb::bson::doc;

#[derive(Database)]
#[database("autofile")]
pub struct MongoDb(mongodb::Client);

#[launch]
fn rocket() -> _ {
    rocket::build().
        attach(MongoDb::init()).
        mount("/", routes![index, health])
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/health")]
pub async fn health(db: &MongoDb) -> Json<HealthResponse> {
    let result = db
        .database("autofile")
        .run_command(doc! { "ping": 1 }, None)
        .await;

    Json(HealthResponse {
        mongo: result.is_ok(),
    })
}

#[derive(serde::Serialize)]
pub struct HealthResponse {
    mongo: bool,
}
