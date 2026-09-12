use std::sync::Arc;

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

use crate::{api, shared::app_state::AppState};

/// Companion to `ApiDoc::openapi()` that assembles the versioned API router and
/// collects the OpenAPI paths contributed by each resource module.
///
/// Every resource module registers its documented handlers via
/// `utoipa_axum::routes!`, so the full route surface is described here.
pub fn build_openapi_router() -> (axum::Router<Arc<AppState>>, utoipa::openapi::OpenApi) {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/about", api::about::routes())
        .nest("/settings", api::app_settings::routes())
        .nest("/auth", api::auth::routes())
        .nest("/cabinets", api::cabinets::routes())
        .nest("/cabinets", api::cabinet_documents::routes())
        .nest("/classifier-blocks", api::classifier_blocks::routes())
        .nest("/document-indexes", api::document_indexes::routes())
        .nest("/document-indexes", api::document_index_templates::routes())
        .nest("/document-indexes", api::document_index_values::routes())
        .nest("/document-types", api::document_types::routes())
        .nest(
            "/document-types-metadata-types",
            api::document_types_metadata_types::routes(),
        )
        .nest("/documents", api::documents::routes())
        .nest("/documents", api::document_file_pages::routes())
        .nest("/documents", api::document_files::routes())
        .nest("/documents", api::document_metadatas::routes())
        .nest("/metadata-types", api::metadata_types::routes())
        .nest("/ping", api::ping::routes())
        .nest("/profile", api::profile::routes())
        .nest("/public", api::public_settings::routes())
        .nest("/users", api::users::routes())
        .nest("/tags", api::tags::routes())
        .nest("/tags", api::tag_documents::routes())
        .split_for_parts()
}

/// Versioned Autofile API description.
///
/// Endpoint paths are contributed by resource routers via
/// `utoipa_axum::router::OpenApiRouter`; this struct carries the document
/// metadata, the bearer security scheme, and explicitly registered schemas
/// (including ones only referenced from query parameters, which utoipa does
/// not auto-collect).
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Autofile API",
        version = env!("CARGO_PKG_VERSION"),
        description = "Autofile document management API."
    ),
    servers((url = "/api/v1", description = "Versioned API base path")),
    modifiers(&SecurityAddon),
    components(schemas(
        crate::shared::util::ApiError,
        crate::domain::users::User,
        crate::domain::app_settings::AppSettings,
        crate::domain::documents::Document,
        crate::domain::documents::DocumentView,
        // Referenced from list query parameters; not auto-collected
        // because the query structs derive IntoParams.
        crate::api::cabinets::CabinetSortField,
        crate::api::cabinet_documents::CabinetDocumentSortField,
        crate::api::classifier_blocks::ClassifierBlockSortField,
        crate::api::document_indexes::DocumentIndexSortField,
        crate::api::document_index_templates::DocumentIndexTemplateSortField,
        crate::api::document_index_values::DocumentIndexValueSortField,
        crate::api::document_types::DocumentTypeSortField,
        crate::api::documents::DocumentSortField,
        crate::api::metadata_types::MetadataTypeSortField,
        crate::api::tag_documents::TagDocumentSortField,
        crate::api::tags::TagSortField,
        crate::application::users::UserSortField,
    ))
)]
pub struct ApiDoc;

/// Registers the `bearer` JWT security scheme used by authenticated endpoints.
/// Send the short-lived access token as `Authorization: Bearer <access_token>`.
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new)
            .add_security_scheme(
                "bearer",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
    }
}
