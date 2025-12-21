use crate::MongoDb;

use serde::{Deserialize, Serialize};

use rocket::http::Status;
use rocket::serde::json::{Json, json, Value};

use mongododm::{CollectionConfig, Index, IndexOption, Indexes, Model, ToRepository};
use mongododm::mongo::bson;
use mongododm::mongo::bson::{oid::ObjectId, doc};
use mongododm::field; // or use mongododm::f for shorthand
use mongododm::mongo::options::{FindOneAndUpdateOptions, ReturnDocument};

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
    description: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateCabinet {
    slug: String,
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateCabinet {
    name: Option<String>,
    description: Option<String>,
}

#[derive(Serialize)]
struct PatchCabinet<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,

    updated_at: i64,
}

impl Cabinet {
    pub fn new(input: CreateCabinet) -> Self {
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
async fn create(db: &MongoDb, input: Json<CreateCabinet>) -> Result<Json<Cabinet>, Status> {
    let repo = db.0.default_database().ok_or(Status::InternalServerError)?.repository::<Cabinet>();

    let cabinet = Cabinet::new(input.into_inner());

    repo.insert_one(&cabinet, None)
        .await
        .map_err(|_| Status::Conflict)?; // e.g. duplicate slug

    Ok(Json(cabinet))
}

#[put("/<slug>", format = "json", data = "<input>")]
async fn update(db: &MongoDb, slug: &str, input: Json<UpdateCabinet>) -> Result<Json<Cabinet>, Status> {
    let repo = db.0.default_database().ok_or(Status::InternalServerError)?.repository::<Cabinet>();

    let now = chrono::Utc::now().timestamp();
    let patch = PatchCabinet {
        name: input.name.as_deref(),
        description: input.description.as_deref(),
        updated_at: now,
    };
    let set_doc = bson::to_document(&patch).map_err(|_| Status::InternalServerError)?;

    let result = repo
        .find_one_and_update(
            doc! { "slug": slug },
            doc! { "$set": set_doc },
            FindOneAndUpdateOptions::builder()
                .return_document(ReturnDocument::After)
                .build(),
        )
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    Ok(Json(result))
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
        rocket.mount("/cabinets", routes![get, create, update])
            .register("/cabinets", catchers![not_found])
            // .manage(MessageList::new(vec![]))
    })
}