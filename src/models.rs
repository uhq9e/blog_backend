// Generated by diesel_ext

use crate::schema::*;
use crate::utils::{datetime_format, naive_date_format};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use chrono::{DateTime, NaiveDate, Utc};

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Identifiable, Deserialize, Serialize)]
pub struct Author {
    pub id: i32,
    pub name: String,
    pub urls: Option<Vec<Option<String>>>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Identifiable, Deserialize, Serialize)]
pub struct ImageCollection {
    pub id: i32,
    pub description: Option<String>,
    #[serde(with = "naive_date_format")]
    pub date: NaiveDate,
}

#[derive(
    Queryable,
    Selectable,
    Insertable,
    Debug,
    Clone,
    Identifiable,
    Associations,
    Deserialize,
    Serialize,
)]
#[diesel(belongs_to(ImageCollection))]
#[diesel(belongs_to(ImageItem))]
#[diesel(primary_key(image_collection_id, image_item_id))]
#[diesel(table_name = image_collections_image_items)]
pub struct ImageCollectionImageItem {
    pub id: i32,
    pub image_collection_id: i32,
    pub image_item_id: i32,
}

#[derive(
    Queryable,
    Selectable,
    Insertable,
    Debug,
    Clone,
    Identifiable,
    Associations,
    Deserialize,
    Serialize,
)]
#[diesel(belongs_to(Author))]
pub struct ImageItem {
    pub id: i32,
    pub urls: Option<Vec<Option<String>>>,
    #[serde(with = "naive_date_format")]
    pub date: NaiveDate,
    pub nsfw: bool,
    pub author_id: Option<i32>,
}

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Identifiable, Serialize, Deserialize)]
pub struct LocalFile {
    pub id: String,
    pub file_name: Option<String>,
    pub path: String,
    #[serde(with = "datetime_format")]
    pub created_at: DateTime<Utc>,
}

#[derive(
    Queryable,
    Selectable,
    Insertable,
    Debug,
    Clone,
    Identifiable,
    Associations,
    Serialize,
    Deserialize,
)]
#[diesel(belongs_to(ImageItem))]
#[diesel(belongs_to(LocalFile))]
#[diesel(table_name = image_items_local_files)]
pub struct ImageItemLocalFile {
    pub id: i32,
    pub image_item_id: i32,
    pub local_file_id: String,
}
