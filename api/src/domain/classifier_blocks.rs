use std::collections::HashMap;

use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::classifier_blocks;

#[derive(Debug, Serialize, Identifiable, PartialEq, Queryable, Selectable)]
#[diesel(table_name = classifier_blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ClassifierBlock {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub order: i32,
    pub rules: diesel_json::Json<ClassifierRules>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_by: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ClassifierRules {
    pub match_patterns: Vec<ClassifierPattern>,
    pub match_actions: HashMap<String, String>,
    pub child_rules: Vec<ClassifierChildRule>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ClassifierPattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ClassifierChildRule {
    pub pattern: ClassifierPattern,
    pub modifiers: Vec<ClassifierModifier>,
    pub actions: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
#[serde(tag = "type")]
pub enum ClassifierModifier {
    #[serde(rename = "metadata")]
    Metadata { to: u32, slug: String },
    #[serde(rename = "month_number")]
    MonthNumber { from: String, to: u32 },
    #[serde(rename = "month_end")]
    MonthEnd { from: String, to: u32 },
    #[serde(rename = "month_start")]
    MonthStart { from: String, to: u32 },
    #[serde(rename = "next_day")]
    NextDay { from: String, to: u32 },
    #[serde(rename = "prev_day")]
    PrevDay { from: String, to: u32 },
    #[serde(rename = "next_month")]
    NextMonth { from: String, to: u32 },
    #[serde(rename = "prev_month")]
    PrevMonth { from: String, to: u32 },
    #[serde(rename = "tax_year")]
    TaxYear { from: String, to: u32 },
    #[serde(rename = "currency")]
    Currency { from: String, to: u32 },
    #[serde(rename = "sprintf")]
    Sprintf {
        from: String,
        to: u32,
        format: String,
    },
    #[serde(rename = "replace")]
    Replace { from: String, to: u32 },
    #[serde(rename = "alnum_sanitize")]
    AlnumSanitize { from: String, to: u32 },
    #[serde(rename = "date_format")]
    DateFormat {
        from: String,
        to: u32,
        format: String,
    },
    #[serde(rename = "add")]
    Add { from: u32, to: u32 },
    #[serde(rename = "sub")]
    Sub { from: u32, to: u32 },
    #[serde(rename = "mul")]
    Mul { from: u32, to: u32 },
    #[serde(rename = "div")]
    Div { from: u32, to: u32 },
}
