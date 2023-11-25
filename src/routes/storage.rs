use aws_sdk_s3::primitives::ByteStream;
use diesel::{delete, insert_into, result::Error as ResultError, ExpressionMethods, QueryDsl};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use md5;
use rocket::{
    delete,
    fs::TempFile,
    get,
    http::{ContentType, Status},
    post,
    serde::json::Json,
    tokio::io,
    Route, State,
};

use crate::{
    db,
    models::*,
    schema,
    utils::{
        response::{DeleteResponse, InsertResponse},
        ApiTokenClaims,
    },
    AppState, BUCKET,
};

const IMAGE_PREFIX: &'static str = "image/";

#[post("/item", data = "<file>")]
async fn create_object(
    app_state: &State<AppState>,
    auth: Option<ApiTokenClaims>,
    db: &State<db::Pool>,
    file: TempFile<'_>,
) -> Result<Json<InsertResponse<String>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let mut data_stream = file.open().await.map_err(|_| Status::InternalServerError)?;
    let mut data_vec: Vec<u8> = vec![];
    io::copy(&mut data_stream, &mut data_vec)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let digest = md5::compute(&data_vec);
    let md5 = format!("{:x}", digest);

    let objs = schema::local_files::table
        .find(&md5)
        .load::<LocalFile>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !objs.is_empty() {
        return Ok(Json(InsertResponse { id: objs[0].id.to_owned() }));
    }

    let filename = if let Some(content_type) = file.content_type() {
        if let Some(ext) = content_type.extension() {
            format!("{}.{}", md5, ext.as_str())
        } else {
            md5.to_owned()
        }
    } else {
        md5.to_owned()
    };

    let key = format!("{}{}", IMAGE_PREFIX, filename);

    let md5_ = md5.to_owned();

    conn.transaction::<(), ResultError, _>(|conn| {
        let content_type = file
            .content_type()
            .or(Some(&ContentType::Binary))
            .unwrap()
            .to_owned();
        let length = file.len().to_owned();
        async move {
            insert_into(schema::local_files::table)
                .values((
                    schema::local_files::id.eq(md5),
                    schema::local_files::file_name.eq(&filename),
                    schema::local_files::path.eq(&key),
                ))
                .execute(conn)
                .await?;

            app_state
                .s3_client
                .put_object()
                .body(ByteStream::from(data_vec))
                .bucket(BUCKET)
                .content_type(content_type.to_string())
                .content_length(length as i64)
                .key(&key)
                .send()
                .await
                .map_err(|_| ResultError::RollbackTransaction)?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(InsertResponse { id: md5_ }))
}

#[get("/item/<id>")]
async fn get_object(db: &State<db::Pool>, id: String) -> Result<Json<LocalFile>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    Ok(Json(
        schema::local_files::table
            .find(id)
            .first::<LocalFile>(&mut conn)
            .await
            .map_err(|err| {
                if let ResultError::NotFound = err {
                    Status::NotFound
                } else {
                    Status::InternalServerError
                }
            })?,
    ))
}

#[delete("/item/<id>")]
async fn delete_object(
    app_state: &State<AppState>,
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: String,
) -> Result<Json<DeleteResponse<String>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let obj = schema::local_files::table
        .find(&id)
        .first::<LocalFile>(&mut conn)
        .await
        .map_err(|err| {
            if let ResultError::NotFound = err {
                Status::NotFound
            } else {
                Status::InternalServerError
            }
        })?;

    let id_ = id.to_owned();

    conn.transaction::<(), ResultError, _>(|conn| {
        async move {
            delete(schema::local_files::table)
                .filter(schema::local_files::id.eq(id_))
                .execute(conn)
                .await?;

            app_state
                .s3_client
                .delete_object()
                .bucket(BUCKET)
                .key(obj.path)
                .send()
                .await
                .map_err(|_| ResultError::RollbackTransaction)?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok(Json(DeleteResponse { id }))
}

pub fn routes() -> Vec<Route> {
    routes![create_object, get_object, delete_object]
}
