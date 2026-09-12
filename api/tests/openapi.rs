use autofile_api::shared::openapi::build_openapi_router;

fn pilot_spec() -> serde_json::Value {
    let (_router, api) = build_openapi_router();
    let json = api.to_json().expect("spec should serialize to JSON");
    serde_json::from_str(&json).expect("spec should parse as JSON")
}

#[test]
fn pilot_paths_are_documented() {
    let spec = pilot_spec();
    let paths = spec["paths"].as_object().expect("spec has paths object");

    for path in [
        "/auth/register",
        "/auth/login",
        "/auth/refresh",
        "/auth/logout",
        "/profile",
        "/profile/password",
        "/settings",
        "/documents",
        "/documents/{id}",
        "/about",
        "/about/license",
        "/ping",
        "/public/settings",
        "/users",
        "/users/{id}",
        "/users/{id}/restore",
        "/users/by-username/{username}",
        "/cabinets",
        "/cabinets/{id}",
        "/cabinets/{cabinet_id}/documents",
        "/cabinets/{cabinet_id}/documents/{document_id}",
        "/tags",
        "/tags/{id}",
        "/tags/{tag_id}/documents",
        "/tags/{tag_id}/documents/{document_id}",
        "/metadata-types",
        "/metadata-types/{id}",
        "/metadata-types/{id}/values",
        "/document-types",
        "/document-types/{id}",
        "/document-types-metadata-types",
        "/document-types-metadata-types/{document_type_id}",
        "/document-types-metadata-types/{document_type_id}/{metadata_type_id}",
        "/documents/{document_id}/metadata",
        "/documents/{document_id}/metadata/{metadata_type_id}",
        "/documents/{document_id}/files",
        "/documents/{document_id}/files/{id}",
        "/documents/{document_id}/files/{id}/thumbnail",
        "/documents/{document_id}/files/{id}/download",
        "/documents/{document_id}/files/{id}/download-ticket",
        "/documents/{document_id}/files/{document_file_id}/pages",
        "/documents/{document_id}/files/{document_file_id}/ocr-pages",
        "/documents/{document_id}/files/{document_file_id}/pages/{page_number}/image",
        "/documents/{id}/thumbnail",
        "/documents/{id}/classify-document",
        "/documents/{id}/test-classifier-block",
        "/documents/{id}/test-template",
        "/documents/{id}/index-values",
        "/classifier-blocks",
        "/classifier-blocks/validate",
        "/classifier-blocks/{id}",
        "/classifier-blocks/{id}/reorder",
        "/document-indexes",
        "/document-indexes/{id}",
        "/document-indexes/{id}/rebuild",
        "/document-indexes/{document_index_id}/templates",
        "/document-indexes/{document_index_id}/templates/{id}",
        "/document-indexes/{document_index_id}/values",
        "/document-indexes/{document_index_id}/values/{id}/ancestors",
    ] {
        assert!(paths.contains_key(path), "path {path} should be documented");
    }
}

#[test]
fn documents_list_declares_query_parameters() {
    let spec = pilot_spec();
    let params = spec["paths"]["/documents"]["get"]["parameters"]
        .as_array()
        .expect("GET /documents has parameters");

    let names: Vec<&str> = params.iter().filter_map(|p| p["name"].as_str()).collect();
    for expected in ["page", "per_page", "q", "text", "sf", "sd", "cabinet_id"] {
        assert!(
            names.contains(&expected),
            "GET /documents should document query parameter {expected}"
        );
    }
}

#[test]
fn authed_operations_require_bearer_security() {
    let spec = pilot_spec();

    let get_documents = &spec["paths"]["/documents"]["get"];
    assert_eq!(
        get_documents["security"],
        serde_json::json!([{"bearer": []}]),
        "GET /documents should require bearer auth"
    );

    let login = &spec["paths"]["/auth/login"]["post"];
    assert!(
        login.get("security").is_none(),
        "POST /auth/login should be public"
    );

    let schemes = &spec["components"]["securitySchemes"]["bearer"];
    assert_eq!(schemes["type"], "http");
    assert_eq!(schemes["scheme"], "bearer");
}

#[test]
fn user_schema_omits_password_hash() {
    let spec = pilot_spec();
    let user = &spec["components"]["schemas"]["User"];
    let properties = user["properties"]
        .as_object()
        .expect("User schema has properties");

    assert!(
        !properties.contains_key("password_hash"),
        "User schema must not expose password_hash"
    );
    assert!(properties.contains_key("username"));
}

#[test]
fn spec_version_tracks_package_version() {
    let spec = pilot_spec();
    assert_eq!(
        spec["info"]["version"].as_str(),
        Some(env!("CARGO_PKG_VERSION")),
        "spec version should come from Cargo.toml"
    );
}

#[test]
fn all_schema_refs_resolve() {
    let spec = pilot_spec();
    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("spec has schemas object");

    fn collect_refs(value: &serde_json::Value, refs: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    if key == "$ref"
                        && let Some(name) = val
                            .as_str()
                            .and_then(|r| r.strip_prefix("#/components/schemas/"))
                    {
                        refs.push(name.to_string());
                    }
                    collect_refs(val, refs);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    collect_refs(item, refs);
                }
            }
            _ => {}
        }
    }

    let mut refs = Vec::new();
    collect_refs(&spec["paths"], &mut refs);
    assert!(!refs.is_empty(), "spec should contain schema references");

    for name in refs {
        assert!(
            schemas.contains_key(&name),
            "schema reference {name} should resolve"
        );
    }
}
