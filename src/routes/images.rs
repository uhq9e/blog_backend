use std::ops::Deref;

use crate::{
    db,
    models::*,
    schema,
    utils::{
        naive_date_format, naive_date_format_option, response::*, result_error_to_status,
        result_error_to_status_failed_dependency, sdk_error_to_status, ApiTokenClaims, Pagination,
        TransactionError,
    },
};
use aws_sdk_s3::operation::put_object::PutObjectError;
use chrono::NaiveDate;
use itertools::izip;
use rocket::{delete, get, http::Status, post, put, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use diesel::{
    delete, insert_into, result::DatabaseErrorKind, update, AsChangeset, BelongingToDsl,
    ExpressionMethods, GroupedBy, QueryDsl, SelectableHelper,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};

#[derive(Serialize)]
struct ImageItemFull {
    #[serde(flatten)]
    image_item: ImageItem,
    author: Option<Author>,
    local_files: Vec<LocalFile>,
}

#[get("/item?<date>&<author_id>&<pg..>")]
async fn list_image_items(
    db: &State<db::Pool>,
    date: Option<String>,
    author_id: Option<i32>,
    pg: Pagination,
) -> Result<Json<ListResponse<ImageItemFull>>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let mut query = schema::image_items::table
        .left_join(schema::authors::table)
        .into_boxed();
    let mut query_count = schema::image_items::table.into_boxed();

    if let Some(val) = date {
        let date = NaiveDate::parse_from_str(val.as_str(), "%Y-%m-%d")
            .map_err(|_| Status::UnprocessableEntity)?;
        query = query.filter(schema::image_items::date.eq(date));
        query_count = query_count.filter(schema::image_items::date.eq(date));
    };
    if let Some(val) = author_id {
        query = query.filter(schema::image_items::author_id.eq(val));
        query_count = query_count.filter(schema::image_items::author_id.eq(val));
    };

    let items_batch: Vec<(ImageItem, Option<Author>)> = query
        .offset(pg.offset)
        .limit(pg.limit)
        .load::<(ImageItem, Option<Author>)>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let image_items: Vec<ImageItem> = items_batch.iter().map(|item| item.0.to_owned()).collect();

    let authors: Vec<Option<Author>> = items_batch.iter().map(|item| item.1.to_owned()).collect();

    let all_local_files: Vec<(ImageItemLocalFile, LocalFile)> =
        ImageItemLocalFile::belonging_to(&image_items)
            .inner_join(schema::local_files::table)
            .select((ImageItemLocalFile::as_select(), LocalFile::as_select()))
            .load(&mut conn)
            .await
            .map_err(|_| Status::InternalServerError)?;

    let local_files: Vec<Vec<LocalFile>> = all_local_files
        .grouped_by(&image_items)
        .into_iter()
        .zip(&image_items)
        .map(|(b, _)| {
            b.into_iter()
                .map(|(_, local_file_item)| local_file_item)
                .collect()
        })
        .collect();

    let results = izip!(&image_items, &authors, &local_files)
        .map(|(image_item, author, local_files)| ImageItemFull {
            image_item: image_item.to_owned(),
            author: author.to_owned(),
            local_files: local_files.to_owned(),
        })
        .collect();

    let count = query_count
        .count()
        .get_result(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(ListResponse::new(results).count(count)))
}

#[get("/item/<id>")]
async fn get_image_item(db: &State<db::Pool>, id: i32) -> Result<Json<ImageItemFull>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::image_items::table
        .find(id)
        .first::<ImageItem>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    let item: (ImageItem, Option<Author>) = schema::image_items::table
        .filter(schema::image_items::id.eq(id))
        .left_join(schema::authors::table)
        .first::<(ImageItem, Option<Author>)>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let local_file_items: Vec<LocalFile> = ImageItemLocalFile::belonging_to(&item.0)
        .inner_join(schema::local_files::table)
        .select(LocalFile::as_select())
        .load::<LocalFile>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(ImageItemFull {
        image_item: item.0,
        author: item.1,
        local_files: local_file_items,
    }))
}

#[derive(Deserialize)]
struct NewImageItemForm {
    author_id: i32,
    local_file_ids: Option<Vec<String>>,
    urls: Vec<String>,
    #[serde(with = "naive_date_format")]
    date: NaiveDate,
}

#[post("/item", data = "<data>")]
async fn create_image_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    data: Json<NewImageItemForm>,
) -> Result<Json<InsertResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let image_item_id = conn
        .transaction::<i32, TransactionError<PutObjectError>, _>(|conn| {
            async move {
                let image_item_id = insert_into(schema::image_items::table)
                    .values((
                        schema::image_items::author_id.eq(data.author_id),
                        schema::image_items::urls.eq(&data.urls),
                        schema::image_items::date.eq(data.date),
                    ))
                    .returning(schema::image_items::id)
                    .get_result::<i32>(conn)
                    .await
                    .map_err(|err| TransactionError::ResultError(err))?;

                if let Some(local_file_ids) = data.local_file_ids.to_owned() {
                    for local_file_id in local_file_ids {
                        schema::local_files::table
                            .find(&local_file_id)
                            .first::<LocalFile>(conn)
                            .await
                            .map_err(|err| TransactionError::ResultError(err))?;

                        insert_into(schema::image_items_local_files::table)
                            .values((
                                schema::image_items_local_files::image_item_id.eq(image_item_id),
                                schema::image_items_local_files::local_file_id.eq(local_file_id),
                            ))
                            .execute(conn)
                            .await
                            .map_err(|err| TransactionError::ResultError(err))?;
                    }
                }

                Ok(image_item_id)
            }
            .scope_boxed()
        })
        .await
        .map_err(|err| match err {
            TransactionError::SdkError(err) => sdk_error_to_status(err),
            TransactionError::ResultError(err) => result_error_to_status_failed_dependency(err),
        })?;

    Ok(Json(InsertResponse { id: image_item_id }))
}

#[derive(Deserialize, Clone, Debug)]
struct UpdateImageItemForm {
    local_file_ids: Option<Vec<String>>,
    urls: Option<Vec<String>>,
    #[serde(with = "naive_date_format_option", default)]
    date: Option<NaiveDate>,
    author_id: Option<i32>,
}

#[derive(AsChangeset)]
#[diesel(table_name = schema::image_items)]
struct ImageItemForUpdate {
    urls: Option<Vec<String>>,
    date: Option<NaiveDate>,
    author_id: Option<i32>,
}

impl From<UpdateImageItemForm> for ImageItemForUpdate {
    fn from(value: UpdateImageItemForm) -> Self {
        Self {
            urls: value.urls,
            date: value.date,
            author_id: value.author_id,
        }
    }
}

impl ImageItemForUpdate {
    fn is_empty(&self) -> bool {
        self.urls.is_none() || self.date.is_none() || self.author_id.is_none()
    }
}

#[put("/item/<id>", data = "<data>")]
async fn update_image_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
    data: Json<UpdateImageItemForm>,
) -> Result<Json<UpdateResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let data = data.deref();

    let update_data: ImageItemForUpdate = data.to_owned().into();

    schema::image_items::table
        .find(id)
        .first::<ImageItem>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    conn.transaction::<(), diesel::result::Error, _>(|conn| {
        let data = data.to_owned();
        async move {
            if !update_data.is_empty() {
                update(schema::image_items::table)
                    .filter(schema::image_items::id.eq(id))
                    .set(update_data)
                    .execute(conn)
                    .await?;
            };

            if let Some(local_file_ids) = data.local_file_ids {
                delete(schema::image_items_local_files::table)
                    .filter(schema::image_items_local_files::image_item_id.eq(id))
                    .execute(conn)
                    .await?;

                insert_into(schema::image_items_local_files::table)
                    .values(
                        local_file_ids
                            .iter()
                            .map(|v| {
                                (
                                    schema::image_items_local_files::image_item_id.eq(id),
                                    schema::image_items_local_files::local_file_id.eq(v),
                                )
                            })
                            .collect::<Vec<_>>(),
                    )
                    .execute(conn)
                    .await?;
            }

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|err| {
        if let diesel::result::Error::DatabaseError(DatabaseErrorKind::ForeignKeyViolation, _) = err
        {
            Status::UnprocessableEntity
        } else {
            Status::InternalServerError
        }
    })?;

    Ok(Json(UpdateResponse { id }))
}

#[delete("/item/<id>")]
async fn delete_image_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
) -> Result<Json<DeleteResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::image_items::table
        .find(id)
        .first::<ImageItem>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    delete(schema::image_items::table.filter(schema::image_items::id.eq(id)))
        .execute(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(DeleteResponse { id }))
}

pub fn routes() -> Vec<Route> {
    routes![
        list_image_items,
        create_image_item,
        get_image_item,
        update_image_item,
        delete_image_item
    ]
}
