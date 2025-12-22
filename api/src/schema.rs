diesel::table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        email -> Varchar,
        display_name -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
    }
}

