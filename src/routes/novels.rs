use crate::{
    db,
    models::*,
    schema,
    utils::{
        parse_order_from_string,
        response::{DeleteResponse, InsertResponse, ListResponse, UpdateResponse},
        result_error_to_status, sdk_error_to_status, ApiTokenClaims, Pagination, TransactionError,
    },
};
use aws_sdk_s3::operation::put_object::PutObjectError;
use diesel::{
    delete, insert_into, query_builder::AsChangeset, result::DatabaseErrorKind, update,
    ExpressionMethods, QueryDsl, TextExpressionMethods,
};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use diesel_order_with_direction::OrderWithDirectionDsl;
use log::info;
use rocket::{delete, get, http::Status, post, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Serialize)]
struct ItemFull {
    #[serde(flatten)]
    novel_item: Novel,
    object: Option<SiteStorage>,
}

#[get(
    "/item?<id>&<title>&<description>&<author_name>&<author_url>&<object_id>&<created_by>&<pg..>"
)]
async fn list_items(
    db: &State<db::Pool>,
    id: Option<i32>,
    title: Option<String>,
    description: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    object_id: Option<i32>,
    created_by: Option<i32>,
    pg: Pagination,
) -> Result<Json<ListResponse<ItemFull>>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let mut query = schema::novels::table
        .left_join(schema::site_storage::table)
        .into_boxed();
    let mut query_count = schema::novels::table.into_boxed();

    // 以id筛选
    if let Some(val) = id {
        query = query.filter(schema::novels::id.eq(val));
        query_count = query_count.filter(schema::novels::id.eq(val));
    }

    // 以title筛选
    if let Some(val) = title {
        query = query.filter(schema::novels::title.like(val.to_owned()));
        query_count = query_count.filter(schema::novels::title.like(val));
    }

    // 以description筛选
    if let Some(val) = description {
        query = query.filter(schema::novels::description.like(val.to_owned()));
        query_count = query_count.filter(schema::novels::description.like(val));
    }

    // 以author_name筛选
    if let Some(val) = author_name {
        query = query.filter(schema::novels::author_name.like(val.to_owned()));
        query_count = query_count.filter(schema::novels::author_name.like(val));
    }

    // 以author_url筛选
    if let Some(val) = author_url {
        query = query.filter(schema::novels::author_url.like(val.to_owned()));
        query_count = query_count.filter(schema::novels::author_url.like(val));
    }

    // 以object_id筛选
    if let Some(val) = object_id {
        query = query.filter(schema::novels::object_id.eq(val));
        query_count = query_count.filter(schema::novels::object_id.eq(val));
    };

    // 以created_by筛选
    if let Some(val) = created_by {
        query = query.filter(schema::novels::created_by.eq(val));
        query_count = query_count.filter(schema::novels::created_by.eq(val));
    };

    // 顺序选择
    for orders in parse_order_from_string(pg.order_by) {
        if let Some(order) = orders {
            query = match order.column.as_str() {
                "id" => query.then_order_by_with_dir(order.direction, schema::novels::id),
                "title" => query.then_order_by_with_dir(order.direction, schema::novels::title),
                "description" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::description)
                }
                "author_name" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::author_name)
                }
                "author_url" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::author_url)
                }
                "object_id" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::object_id)
                }
                "created_by" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::created_by)
                }
                "created_at" => {
                    query.then_order_by_with_dir(order.direction, schema::novels::created_at)
                }
                _ => query,
            }
        }
    }

    let items_batch: Vec<(Novel, Option<SiteStorage>)> = query
        .offset(pg.offset)
        .limit(pg.limit)
        .load::<(Novel, Option<SiteStorage>)>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let results = items_batch
        .iter()
        .map(|(novel, site_storage)| ItemFull {
            novel_item: novel.to_owned(),
            object: site_storage.to_owned(),
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
async fn get_item(db: &State<db::Pool>, id: i32) -> Result<Json<ItemFull>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::novels::table
        .find(id)
        .first::<Novel>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    let item: (Novel, Option<SiteStorage>) = schema::novels::table
        .filter(schema::novels::id.eq(id))
        .left_join(schema::site_storage::table)
        .first::<(Novel, Option<SiteStorage>)>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(ItemFull {
        novel_item: item.0,
        object: item.1,
    }))
}

#[derive(Deserialize)]
struct NewItemForm {
    title: String,
    description: Option<String>,
    author_name: String,
    author_url: Option<String>,
    object_id: i32,
    created_by: Option<i32>,
}

#[post("/item", data = "<data>")]
async fn create_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    data: Json<NewItemForm>,
) -> Result<Json<InsertResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let new_item_id = conn
        .transaction::<i32, TransactionError<PutObjectError>, _>(|conn| {
            async move {
                let new_item_id = insert_into(schema::novels::table)
                    .values((
                        schema::novels::title.eq(&data.title),
                        schema::novels::description.eq(&data.description),
                        schema::novels::author_name.eq(&data.author_name),
                        schema::novels::author_url.eq(&data.author_url),
                        schema::novels::object_id.eq(data.object_id),
                        schema::novels::created_by.eq(data.created_by),
                    ))
                    .returning(schema::novels::id)
                    .get_result::<i32>(conn)
                    .await
                    .map_err(|err| TransactionError::ResultError(err))?;

                Ok(new_item_id)
            }
            .scope_boxed()
        })
        .await
        .map_err(|err| match err {
            TransactionError::SdkError(err) => sdk_error_to_status(err),
            TransactionError::ResultError(err) => result_error_to_status(err),
        })?;

    info!("Create novel item: {}", new_item_id);

    Ok(Json(InsertResponse { id: new_item_id }))
}

#[derive(AsChangeset, Deserialize, Clone, Debug)]
#[diesel(table_name = schema::novels)]
struct ItemForUpdate {
    title: Option<String>,
    description: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    object_id: Option<i32>,
    created_by: Option<i32>,
}

impl ItemForUpdate {
    fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.author_name.is_none()
            && self.author_url.is_none()
            && self.object_id.is_none()
            && self.created_by.is_none()
    }
}

#[put("/item/<id>", data = "<data>")]
async fn update_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
    data: Json<ItemForUpdate>,
) -> Result<Json<UpdateResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let data = data.deref();

    schema::novels::table
        .find(id)
        .first::<Novel>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    if !data.is_empty() {
        update(schema::novels::table)
            .filter(schema::novels::id.eq(id))
            .set(data)
            .execute(&mut conn)
            .await
            .map_err(|err| {
                if let diesel::result::Error::DatabaseError(
                    DatabaseErrorKind::ForeignKeyViolation,
                    _,
                ) = err
                {
                    Status::UnprocessableEntity
                } else {
                    Status::InternalServerError
                }
            })?;
    };

    info!("Update novel item: {}", id);

    Ok(Json(UpdateResponse { id }))
}

#[delete("/item/<id>")]
async fn delete_item(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
) -> Result<Json<DeleteResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::novels::table
        .find(id)
        .first::<Novel>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    delete(schema::novels::table.filter(schema::novels::id.eq(id)))
        .execute(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    info!("Delete novel item: {}", id);

    Ok(Json(DeleteResponse { id }))
}

pub fn routes() -> Vec<Route> {
    routes![list_items, get_item, create_item, update_item, delete_item]
}
