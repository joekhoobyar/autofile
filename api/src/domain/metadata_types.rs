use chrono::{DateTime, Utc};
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::sql_types::Varchar;
use diesel::{AsExpression, FromSqlRow};
use serde::{Deserialize, Serialize};
use std::io::Write;

use crate::schema::metadata_types;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = metadata_types)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct MetadataType {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub data_type: DataType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = Varchar)]
#[serde(rename_all = "lowercase")]
pub enum DataType {
    String,
    Date,
    Lookup,
}

impl DataType {
    fn as_str(self) -> &'static str {
        match self {
            DataType::String => "string",
            DataType::Date => "date",
            DataType::Lookup => "lookup",
        }
    }
}

impl ToSql<Varchar, Pg> for DataType {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Varchar, Pg> for DataType {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let value = <String as FromSql<Varchar, Pg>>::from_sql(bytes)?;
        match value.as_str() {
            "string" => Ok(DataType::String),
            "date" => Ok(DataType::Date),
            "lookup" => Ok(DataType::Lookup),
            other => Err(format!("Unrecognized data_type: {other}").into()),
        }
    }
}
