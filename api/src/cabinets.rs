use crate::MongoDb;

use serde::{Deserialize, Serialize};
use rocket::serde::json::{Json, json, Value};
use mongododm::{
    CollectionConfig, Index, IndexOption, Indexes, Model, ToRepository, 
};
use mongododm::mongo::bson::doc;
use mongododm::field; // or use mongododm::f for shorthand

pub struct CabinetColl;
impl CollectionConfig for CabinetColl {
    fn collection_name() -> &'static str { "cabinets" }

    fn indexes() -> Indexes {
        Indexes::new()
            .with(Index::new("slug").with_option(IndexOption::Unique))
            .with(Index::new(field!(name in Cabinet)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Cabinet {
    slug: String,
    name: String,
    description: String,
}

impl Model for Cabinet {
    type CollConf = CabinetColl;
}

#[get("/<slug>")]
pub async fn get(db: &MongoDb, slug: &str) -> Option<Json<Cabinet>> {
    let repo = db.0.default_database()?.repository::<Cabinet>();

    // compile-time checked field name
    let found = repo
        .find_one(doc! { field!(slug in Cabinet): slug }, None)
        .await
        .ok()??;

    Some(Json(found))
}

#[catch(404)]
fn not_found() -> Value {
    json!({
        "status": "error",
        "reason": "No such cabinet."
    })
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile Cabinets", |rocket| async {
        rocket.mount("/cabinets", routes![get])
            .register("/cabinets", catchers![not_found])
            // .manage(MessageList::new(vec![]))
    })
}