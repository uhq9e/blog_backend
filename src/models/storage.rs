use crate::schema::*;
use crate::utils::datetime_format;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Identifiable, Deserialize, Serialize)]
#[diesel(table_name = site_storage)]
pub struct SiteStorage {
    pub id: i32,
    pub file_name: String,
    pub key: String,
    pub size: i64,
    pub hash: String,
    pub kind: i16,
    pub mime_type: String,
    pub created_by: Option<i32>,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}
