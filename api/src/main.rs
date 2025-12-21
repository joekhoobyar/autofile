#[macro_use] extern crate rocket;

use rocket_db_pools::{mongodb, Database};
use rocket::serde::json::Json;
use mongodb::bson::doc;

#[derive(Database)]
#[database("autofile")]
pub struct MongoDb(mongodb::Client);

mod cabinets;

#[launch]
fn rocket() -> _ {
    rocket::build().
        attach(MongoDb::init()).
        mount("/", routes![index, health_ready]).
        attach(cabinets::stage())
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/health/ready")]
pub async fn health_ready(db: &MongoDb) -> Json<ReadyResponse> {
    let mongo_ok = db
        .database("autofile")
        .run_command(doc! { "ping": 1 }, None)
        .await.is_ok();

    Json(ReadyResponse {
        ok : mongo_ok,
        mongo : mongo_ok,
    })
}

#[derive(serde::Serialize)]
pub struct ReadyResponse {
    ok: bool,
    mongo: bool,
}