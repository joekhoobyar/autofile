use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::Serialize;

use crate::schema::cabinet_documents;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(belongs_to(Cabinet))]
#[diesel(belongs_to(Document))]
#[diesel(table_name = cabinet_documents)]
#[diesel(primary_key(cabinet_id, document_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct CabinetDocument {
    pub cabinet_id: i64,
    pub document_id: i64,
    pub updated_at: DateTime<Utc>,
    pub updated_by: i64,
}
