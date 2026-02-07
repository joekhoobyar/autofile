use crate::Db;
use crate::auth::AuthUser;
use crate::schema::{documents, document_files};
use crate::util::{diesel_to_http, err, ApiResult};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::{Form, FromForm};
use rocket::fs::TempFile;
use rocket::{State};
use rocket::http::Status;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::io::AsyncReadExt;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Document {
    id: i64,

    title: String,
    document_type_id: i64,

    created_by: i64,
    created_at: DateTime<Utc>,
    updated_by: i64,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Insertable)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocument {
    title: String,
    document_type_id: i64,
}

#[derive(Debug, Deserialize, AsChangeset)]
#[diesel(table_name = documents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct DocumentChangeset {
    title: Option<String>,
    document_type_id: Option<i64>,
}

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentFile {
    id: i64,
    document_id: i64,
    s3_prefix: String,
    filename: String,
    content_type: Option<String>,
    size: i64,
    created_at: DateTime<Utc>,
    created_by: i64,
    updated_at: DateTime<Utc>,
    updated_by: i64,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = document_files)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewDocumentFile {
    document_id: i64,
    s3_prefix: String,
    filename: String,
    content_type: Option<String>,
    size: i64,
    created_by: i64,
    updated_by: i64,
}

#[derive(FromForm)]
pub struct ListDocumentsQuery {
    // 1-based page number
    pub page: Option<i64>,
    // items per page (cap it)
    pub per_page: Option<i64>,
    // optional substring search
    pub q: Option<String>,
    document_type_id: Option<i64>,
}

#[derive(FromForm)]
pub struct DocumentFormData<'r> {
    title: String,
    document_type_id: i64,
    file: Option<TempFile<'r>>,
}

#[get("/<id>")]
pub async fn get(mut db: Connection<Db>, _user: AuthUser, id: i64) -> ApiResult<Json<Document>> {
    let row = documents::table
        .find(id)
        .select(Document::as_select())
        .first::<Document>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to fetch document"))?;

    Ok(Json(row))
}

#[post("/", data = "<form>")]
async fn create(
    mut db: Connection<Db>,
    user: AuthUser,
    mut form: Form<DocumentFormData<'_>>,
    s3_client: &State<aws_sdk_s3::Client>,
    s3_bucket: &State<String>,
) -> ApiResult<Json<Document>> {
    // Handle optional file upload
    let file_info = if let Some(ref mut file) = form.file {
        // Extract file metadata
        let filename = file.name()
            .ok_or_else(|| err(Status::BadRequest, "file has no filename"))?
            .to_string();
        let content_type = file.content_type().map(|ct| ct.to_string());

        // Read file content into memory
        let mut file_data = Vec::new();
        file.open().await
            .map_err(|e| err(Status::InternalServerError, &format!("failed to open temp file: {}", e)))?
            .read_to_end(&mut file_data)
            .await
            .map_err(|e| err(Status::InternalServerError, &format!("failed to read file: {}", e)))?;

        let file_size = file_data.len() as i64;

        // Generate UUID for s3_prefix
        let s3_prefix = Uuid::new_v4().to_string();
        let s3_key = format!("{}/{}", s3_prefix, filename);

        // Upload to S3
        crate::s3::upload_to_s3(
            s3_client.inner(),
            s3_bucket.as_str(),
            &s3_key,
            &file_data,
            content_type.as_deref(),
        )
        .await
        .map_err(|e| err(Status::InternalServerError, &format!("S3 upload failed: {}", e)))?;

        Some((s3_prefix, filename, content_type, file_size))
    } else {
        None
    };

    // Clone file_info for potential cleanup in error path
    let file_info_for_cleanup = file_info.clone();

    // Begin database transaction
    let result = db.build_transaction()
        .run::<_, diesel::result::Error, _>(|conn| {
            Box::pin(async move {
                // Insert document record
                let inserted_document: Document = diesel::insert_into(documents::table)
                    .values((
                        documents::title.eq(&form.title),
                        documents::document_type_id.eq(form.document_type_id),
                        documents::created_by.eq(user.user_id),
                        documents::updated_by.eq(user.user_id),
                    ))
                    .returning(Document::as_returning())
                    .get_result(conn)
                    .await?;

                // If file was uploaded, insert document_files record
                if let Some((s3_prefix, filename, content_type, file_size)) = file_info {
                    diesel::insert_into(document_files::table)
                        .values(&NewDocumentFile {
                            document_id: inserted_document.id,
                            s3_prefix,
                            filename,
                            content_type,
                            size: file_size,
                            created_by: user.user_id,
                            updated_by: user.user_id,
                        })
                        .execute(conn)
                        .await?;
                }

                Ok(inserted_document)
            })
        })
        .await;

    match result {
        Ok(document) => Ok(Json(document)),
        Err(e) => {
            // On transaction failure, attempt S3 cleanup (best-effort)
            if let Some((s3_prefix, filename, _, _)) = file_info_for_cleanup {
                let s3_key = format!("{}/{}", s3_prefix, filename);
                let _ = crate::s3::delete_from_s3(
                    s3_client.inner(),
                    s3_bucket.as_str(),
                    &s3_key,
                ).await;
            }
            Err(err(diesel_to_http(e), "failed to create document"))
        }
    }
}

#[patch("/<id>", format = "json", data = "<input>")]
async fn update(mut db: Connection<Db>, user: AuthUser, id: i64, input: Json<DocumentChangeset>) -> ApiResult<Json<Document>> {
    // Update + return the updated row in one round-trip.
    let updated: Document =
        diesel::update(documents::table.filter(documents::id.eq(id)))
            .set((
                &input.into_inner(),
                documents::updated_by.eq(user.user_id),
                documents::updated_at.eq(Utc::now()),
            ))
            .returning(Document::as_returning())
            .get_result(&mut db)
            .await
            .map_err(|e| err(diesel_to_http(e), "failed to update document"))?;

    Ok(Json(updated))
}

#[get("/?<params..>")]
pub async fn list(mut db: Connection<Db>, _user: AuthUser, params: ListDocumentsQuery) -> ApiResult<Json<Vec<Document>>> {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(50).clamp(1, 200);
    let offset = (page - 1) * per_page;

    // Start with a boxed query so we can conditionally add filters.
    let mut query = documents::table.into_boxed();

    // Optional search: case-insensitive substring on slug/name/description
    if let Some(q) = params.q.as_deref().filter(|s| !s.is_empty()) {
        let pattern = format!("%{}%", q);
        query = query.filter(
            documents::title.ilike(pattern)
        );
    }

    // Filter by document type
    if let Some(id) = params.document_type_id {
        query = query.filter(documents::document_type_id.eq(id));
    }

    let rows = query
        .order(documents::id.desc())
        .limit(per_page)
        .offset(offset)
        .select(Document::as_select())
        .load::<Document>(&mut db)
        .await
        .map_err(|e| err(diesel_to_http(e), "failed to list documents"))?;

    Ok(Json(rows))
}

pub fn stage() -> rocket::fairing::AdHoc {
    rocket::fairing::AdHoc::on_ignite("Autofile documents", |rocket| async {
        rocket.mount("/documents", routes![list, get, create, update])
    })
}
