// @generated automatically by Diesel CLI.

diesel::table! {
    cabinet_documents (cabinet_id, document_id) {
        cabinet_id -> Int8,
        document_id -> Int8,
        updated_at -> Timestamptz,
        updated_by -> Int8,
    }
}

diesel::table! {
    cabinets (id) {
        id -> Int8,
        slug -> Varchar,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        parent_id -> Nullable<Int8>,
        created_by -> Int8,
        updated_by -> Int8,
    }
}

diesel::table! {
    document_files (id) {
        id -> Int8,
        document_id -> Int8,
        #[max_length = 36]
        s3_prefix -> Varchar,
        #[max_length = 512]
        filename -> Varchar,
        #[max_length = 255]
        content_type -> Nullable<Varchar>,
        size -> Int8,
        created_at -> Timestamptz,
        created_by -> Int8,
        updated_at -> Timestamptz,
        updated_by -> Int8,
    }
}

diesel::table! {
    document_metadatas (document_id, metadata_type_id) {
        document_id -> Int8,
        metadata_type_id -> Int8,
        value -> Varchar,
        created_at -> Timestamptz,
        created_by -> Int8,
        updated_at -> Timestamptz,
        updated_by -> Int8,
    }
}

diesel::table! {
    document_types (id) {
        id -> Int8,
        slug -> Varchar,
        name -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        created_by -> Int8,
        updated_by -> Int8,
    }
}

diesel::table! {
    document_types_metadata_types (document_type_id, metadata_type_id) {
        document_type_id -> Int8,
        metadata_type_id -> Int8,
        required -> Bool,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    documents (id) {
        id -> Int8,
        title -> Varchar,
        document_type_id -> Int8,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        created_by -> Int8,
        updated_by -> Int8,
        #[max_length = 64]
        s3_thumbnail -> Nullable<Varchar>,
    }
}

diesel::table! {
    metadata_types (id) {
        id -> Int8,
        slug -> Varchar,
        name -> Varchar,
        data_type -> Varchar,
        description -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        options -> Nullable<Jsonb>,
        created_by -> Int8,
        updated_by -> Int8,
    }
}

diesel::table! {
    tag_documents (tag_id, document_id) {
        tag_id -> Int8,
        document_id -> Int8,
        updated_at -> Timestamptz,
        updated_by -> Int8,
    }
}

diesel::table! {
    tags (id) {
        id -> Int8,
        slug -> Varchar,
        name -> Varchar,
        #[max_length = 6]
        color -> Bpchar,
        created_at -> Timestamptz,
        created_by -> Int8,
        updated_at -> Timestamptz,
        updated_by -> Int8,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        username -> Text,
        display_name -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        email -> Varchar,
        password_hash -> Text,
        password_changed_at -> Timestamptz,
    }
}

diesel::joinable!(cabinet_documents -> cabinets (cabinet_id));
diesel::joinable!(cabinet_documents -> documents (document_id));
diesel::joinable!(cabinet_documents -> users (updated_by));
diesel::joinable!(document_files -> documents (document_id));
diesel::joinable!(document_metadatas -> documents (document_id));
diesel::joinable!(document_metadatas -> metadata_types (metadata_type_id));
diesel::joinable!(document_types_metadata_types -> document_types (document_type_id));
diesel::joinable!(document_types_metadata_types -> metadata_types (metadata_type_id));
diesel::joinable!(documents -> document_types (document_type_id));
diesel::joinable!(tag_documents -> documents (document_id));
diesel::joinable!(tag_documents -> tags (tag_id));
diesel::joinable!(tag_documents -> users (updated_by));

diesel::allow_tables_to_appear_in_same_query!(
    cabinet_documents,
    cabinets,
    document_files,
    document_metadatas,
    document_types,
    document_types_metadata_types,
    documents,
    metadata_types,
    tag_documents,
    tags,
    users,
);
