use crate::MongoDb;

use serde::{Deserialize, Serialize};

use rocket::http::Status;
use rocket::serde::json::{Json, json, Value};

use mongododm::{
    CollectionConfig, Index, IndexOption, Indexes, Model, ToRepository, 

};
use mongododm::mongo::bson::{oid::ObjectId, doc};
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
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,

    created_at: i64,
    updated_at: i64,

    slug: String,
    name: String,
    description: String,
}

#[derive(Deserialize)]
pub struct CabinetInput {
    slug: String,
    name: String,
    description: String,
}

impl Cabinet {
    pub fn new(input: CabinetInput) -> Self {
        let now = chrono::Utc::now().timestamp();
        Cabinet {
            id: None,
            created_at: now,
            updated_at: now,
            slug: input.slug,
            name: input.name,
            description: input.description,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

impl Model for Cabinet {
    type CollConf = CabinetColl;
}

#[get("/<slug>", format = "json")]
pub async fn get(db: &MongoDb, slug: &str) -> Option<Json<Cabinet>> {
    let repo = db.0.default_database()?.repository::<Cabinet>();

    // compile-time checked field name
    let found = repo
        .find_one(doc! { field!(slug in Cabinet): slug }, None)
        .await
        .ok()??;

    Some(Json(found))
}

#[post("/", format = "json", data = "<input>")]
async fn create(db: &MongoDb, input: Json<CabinetInput>) -> Result<Json<Cabinet>, Status> {
    let repo = db.0.default_database().ok_or(Status::InternalServerError)?.repository::<Cabinet>();

    let cabinet = Cabinet::new(input.into_inner());

    repo.insert_one(&cabinet, None)
        .await
        .map_err(|_| Status::Conflict)?; // e.g. duplicate slug

    Ok(Json(cabinet))
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
        rocket.mount("/cabinets", routes![get, create])
            .register("/cabinets", catchers![not_found])
            // .manage(MessageList::new(vec![]))
    })
}