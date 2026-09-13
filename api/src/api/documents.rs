use std::sync::Arc;

use crate::application::document_files::{
    BufferedDocumentFileUpload, buffer_document_file_field, cleanup_buffered_document_file_upload,
};
use crate::application::document_index_documents::enqueue_document_index_document_updates;
use crate::application::documents::{
    CreateDocumentInput, ListDocumentsQuery, TestClassifierBlockResponse, TestTemplateResponse,
    create_document, delete_document, enqueue_document_classification,
    enqueue_document_file_page_processing, enqueue_document_thumbnail_generation,
    get_document_thumbnail_metadata, get_document_view, list_document_index_values, list_documents,
    test_classifier_block as test_classifier_block_workflow,
    test_template as test_template_workflow, update_document,
};
use crate::domain::document_indexes::DocumentIndexValue;
use crate::domain::documents::{Document, DocumentChangeset, DocumentView};
use crate::shared::app_state::AppState;
use crate::shared::auth::AuthUser;
use crate::shared::config::MAX_UPLOAD_MULTIPART_OVERHEAD_BYTES;
use crate::shared::errors::ApiError;
use crate::shared::extractors::DbConn;
use crate::shared::responses::ResourceList;
use crate::shared::s3::serve_s3_file;

use axum::extract::DefaultBodyLimit;
use serde::Deserialize;

use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::HeaderMap,
    response::Response,
};
use utoipa_axum::{router::OpenApiRouter, routes};

/// Documented shape of the `multipart/form-data` body accepted by document
/// creation. This type is never constructed; it only describes the wire format
/// because the handler parses the multipart stream manually.
#[derive(utoipa::ToSchema)]
#[allow(dead_code)]
struct CreateDocumentMultipart {
    /// Document title (required).
    title: String,
    /// Owning document type ID (required).
    document_type_id: i64,
    /// Optional uploaded file bytes.
    #[schema(value_type = Option<String>, format = Binary)]
    file: Option<String>,
    /// Optional per-upload virus scan selection. Ignored when virus scanning is disabled.
    virus_scan: Option<bool>,
}

struct ParsedMultipart {
    title: Option<String>,
    document_type_id: Option<i64>,
    file_temp: Option<BufferedDocumentFileUpload>,
    virus_scan: Option<bool>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
struct TestClassifierBlockInput {
    classifier_block_id: i64,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
struct TestTemplateInput {
    template: String,
}

async fn parse_create_multipart(
    multipart: &mut Multipart,
    max_bytes: u64,
) -> Result<ParsedMultipart, ApiError> {
    let mut title: Option<String> = None;
    let mut document_type_id: Option<i64> = None;
    let mut file_temp: Option<BufferedDocumentFileUpload> = None;
    let mut virus_scan: Option<bool> = None;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::bad_request(&format!("Failed to read multipart field: {}", e)))?
    {
        let field_name = field
            .name()
            .ok_or_else(|| ApiError::bad_request("Field missing name"))?
            .to_string();

        match field_name.as_str() {
            "title" => {
                let value = field
                    .text()
                    .await
                    .map_err(|e| ApiError::bad_request(&format!("Failed to read title: {}", e)))?;
                title = Some(value);
            }
            "document_type_id" => {
                let value = field.text().await.map_err(|e| {
                    ApiError::bad_request(&format!("Failed to read document_type_id: {}", e))
                })?;
                document_type_id = Some(
                    value
                        .parse::<i64>()
                        .map_err(|_| ApiError::bad_request("Invalid document_type_id"))?,
                );
            }
            "file" => {
                if let Some(upload) = &file_temp {
                    cleanup_buffered_document_file_upload(upload).await;
                    return Err(ApiError::bad_request("Only one file upload is supported"));
                }
                file_temp = Some(buffer_document_file_field(&mut field, max_bytes).await?);
            }
            "virus_scan" | "scan_for_viruses" => {
                let value = field.text().await.map_err(|e| {
                    ApiError::bad_request(&format!("Failed to read virus_scan: {}", e))
                })?;
                virus_scan = Some(parse_multipart_bool(&value, "virus_scan")?);
            }
            _ => {
                // Ignore unknown fields
            }
        }
    }

    Ok(ParsedMultipart {
        title,
        document_type_id,
        file_temp,
        virus_scan,
    })
}

fn parse_multipart_bool(value: &str, field_name: &str) -> Result<bool, ApiError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(ApiError::bad_request(&format!("Invalid {field_name}"))),
    }
}

#[utoipa::path(
    get,
    path = "/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document detail", body = DocumentView),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
    )
)]
pub async fn get_by_id(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<DocumentView>, ApiError> {
    let document_view = get_document_view(&mut db, id).await?;
    Ok(Json(document_view))
}

#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document deleted"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
    )
)]
pub async fn delete(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    delete_document(state, &mut db, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/process-file-pages",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "File page processing job enqueued"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 500, description = "Failed to enqueue job", body = ApiError),
    )
)]
pub async fn process_file_pages(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    enqueue_document_file_page_processing(state, &mut db, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/generate-thumbnail",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Thumbnail job enqueued"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 500, description = "Failed to enqueue job", body = ApiError),
    )
)]
pub async fn generate_thumbnail(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    enqueue_document_thumbnail_generation(state, &mut db, id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/classify-document",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Classification job enqueued"),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 500, description = "Failed to enqueue job", body = ApiError),
    )
)]
pub async fn classify_document(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<()>, ApiError> {
    enqueue_document_classification(state, &mut db, id, user.user_id).await?;
    Ok(Json(()))
}

#[utoipa::path(
    post,
    path = "/{id}/test-classifier-block",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    request_body = TestClassifierBlockInput,
    responses(
        (status = 200, description = "Actions the block would compute for this document", body = TestClassifierBlockResponse),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document or classifier block not found", body = ApiError),
        (status = 500, description = "Failed to compute actions", body = ApiError),
    )
)]
async fn test_classifier_block(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TestClassifierBlockInput>,
) -> Result<Json<TestClassifierBlockResponse>, ApiError> {
    Ok(Json(
        test_classifier_block_workflow(&mut db, id, input.classifier_block_id).await?,
    ))
}

#[utoipa::path(
    post,
    path = "/{id}/test-template",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    request_body = TestTemplateInput,
    responses(
        (status = 200, description = "Rendered template, or the render error; template errors return 200 with an error field", body = TestTemplateResponse),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 500, description = "Failed to build template view", body = ApiError),
    )
)]
async fn test_template(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<TestTemplateInput>,
) -> Result<Json<TestTemplateResponse>, ApiError> {
    Ok(Json(
        test_template_workflow(&mut db, id, &input.template).await?,
    ))
}

/**
 * thumbnail_get
 *
 * This handler serves the thumbnail image for a document, streaming it directly from S3.
 * It supports conditional GET with If-Modified-Since header to optimize caching.
 */
#[utoipa::path(
    get,
    path = "/{id}/thumbnail",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document thumbnail bytes", content_type = "image/png", body = Vec<u8>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Thumbnail not available", body = ApiError),
    )
)]
pub async fn thumbnail_get(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let (s3_key, updated_at) = get_document_thumbnail_metadata(&mut db, id).await?;

    serve_s3_file(
        state.as_ref(),
        &headers,
        &s3_key,
        Some(updated_at),
        "Thumbnail not available",
        Some("image/png"),
    )
    .await
}

#[utoipa::path(
    post,
    path = "/",
    tag = "documents",
    security(("bearer" = [])),
    request_body(
        content = CreateDocumentMultipart,
        content_type = "multipart/form-data",
        description = "Document fields with an optional uploaded file"
    ),
    responses(
        (status = 200, description = "Created document", body = Document),
        (status = 400, description = "Invalid request, e.g. missing title or document_type_id", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document type not found", body = ApiError),
        (status = 422, description = "Unprocessable request", body = ApiError),
    )
)]
async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    mut multipart: Multipart,
) -> Result<Json<Document>, ApiError> {
    let ParsedMultipart {
        title,
        document_type_id,
        file_temp,
        virus_scan,
    } = parse_create_multipart(&mut multipart, state.max_upload_bytes as u64).await?;

    let document = create_document(
        state,
        &mut db,
        user.user_id,
        CreateDocumentInput {
            title,
            document_type_id,
            file_upload: file_temp,
            virus_scan,
        },
    )
    .await?;

    Ok(Json(document))
}

#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    request_body = DocumentChangeset,
    responses(
        (status = 200, description = "Updated document", body = Document),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
        (status = 422, description = "Unprocessable request", body = ApiError),
    )
)]
async fn update(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
    Json(input): Json<DocumentChangeset>,
) -> Result<Json<Document>, ApiError> {
    let updated = update_document(user.user_id, &mut db, id, input).await?;

    // Enqueue jobs to update document indexes for this document, as the tags may be used in index rules.
    enqueue_document_index_document_updates(id, state).await?;

    Ok(Json(updated))
}

#[utoipa::path(
    get,
    path = "/",
    tag = "documents",
    security(("bearer" = [])),
    params(ListDocumentsQuery),
    responses(
        (status = 200, description = "Paginated document list", body = ResourceList<DocumentView>),
        (status = 400, description = "Invalid query", body = ApiError),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
    )
)]
pub async fn list(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Query(params): Query<ListDocumentsQuery>,
) -> Result<Json<ResourceList<DocumentView>>, ApiError> {
    Ok(Json(list_documents(&mut db, params).await?))
}

#[utoipa::path(
    get,
    path = "/{id}/index-values",
    tag = "documents",
    security(("bearer" = [])),
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Index values assigned to the document", body = Vec<DocumentIndexValue>),
        (status = 401, description = "Missing or invalid access token", body = ApiError),
        (status = 403, description = "Password change required", body = ApiError),
        (status = 404, description = "Document not found", body = ApiError),
    )
)]
pub async fn list_index_values(
    _user: AuthUser,
    DbConn(mut db): DbConn,
    Path(id): Path<i64>,
) -> Result<Json<Vec<DocumentIndexValue>>, ApiError> {
    let items = list_document_index_values(&mut db, id).await?;
    Ok(Json(items))
}

pub fn routes(max_upload_bytes: usize) -> OpenApiRouter<Arc<AppState>> {
    OpenApiRouter::new()
        .routes(routes!(list))
        .routes(routes!(create))
        // Coarse backstop: the precise per-file limit is enforced while
        // streaming in the handler, so the layer allows multipart overhead
        // on top of the configured file limit.
        .layer(DefaultBodyLimit::max(
            max_upload_bytes.saturating_add(MAX_UPLOAD_MULTIPART_OVERHEAD_BYTES),
        ))
        .routes(routes!(get_by_id))
        .routes(routes!(update))
        .routes(routes!(delete))
        .routes(routes!(classify_document))
        .routes(routes!(test_classifier_block))
        .routes(routes!(test_template))
        .routes(routes!(list_index_values))
        .routes(routes!(thumbnail_get))
        .routes(routes!(generate_thumbnail))
        .routes(routes!(process_file_pages))
}
