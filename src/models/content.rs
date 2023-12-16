use crate::schema::*;
use crate::utils::datetime_format;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, Utc};

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Identifiable, Deserialize, Serialize)]
pub struct Novel {
    pub id: i32,
    pub title: String,
    pub description: Option<String>,
    pub object_id: Option<i32>,
    pub created_by: Option<i32>,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}
