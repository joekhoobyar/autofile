use crate::Db;
use crate::schema::{document_types};
use crate::util::{diesel_to_http, err, ApiResult};

use serde::{Deserialize, Serialize};

use rocket::serde::json::Json;
use rocket::form::FromForm;

use rocket_db_pools::Connection;
use rocket_db_pools::diesel::prelude::*;

use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Identifiable, Associations, Queryable, Selectable)]
#[diesel(belongs_to(DocumentType))]
#[diesel(belongs_to(MetadataType))]
#[diesel(table_name = document_types_metadata_types)]
#[diesel(primary_key(document_type_id, metadata_type_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DocumentTypeMetadataType {
    document_type_id: i64,
    metadata_type_id: i64,
    required: bool,
}
