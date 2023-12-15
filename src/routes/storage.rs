use std::io::Cursor;

use aws_sdk_s3::{
    operation::{delete_object::DeleteObjectError, put_object::PutObjectError},
    primitives::ByteStream,
};
use diesel::{delete, insert_into, ExpressionMethods, QueryDsl};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use image::io::Reader as ImageReader;
use image::ImageOutputFormat;
use log::info;
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
        result_error_to_status, transaction_error_to_status, ApiTokenClaims, TransactionError,
    },
    AppState, BUCKET,
};

const IMAGE_PREFIX: &'static str = "image/";

impl<T> From<diesel::result::Error> for TransactionError<T> {
    fn from(value: diesel::result::Error) -> Self {
        Self::ResultError(value)
    }
}

#[post("/image/item", data = "<file>")]
async fn create_object(
    app_state: &State<AppState>,
    auth: Option<ApiTokenClaims>,
    db: &State<db::Pool>,
    file: TempFile<'_>,
) -> Result<Json<InsertResponse<String>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let binary = ContentType::Binary;
    let content_type = file.content_type().or(Some(&binary)).unwrap();

    if content_type.top().ne("image") {
        return Err(Status::UnprocessableEntity);
    };

    let mut data_stream = file.open().await.map_err(|_| Status::BadRequest)?;
    let mut data_vec: Vec<u8> = vec![];
    io::copy(&mut data_stream, &mut data_vec)
        .await
        .map_err(|_| Status::BadRequest)?;

    let mut new_data_vec: Vec<u8> = Vec::new();
    ImageReader::new(Cursor::new(data_vec))
        .with_guessed_format()
        .map_err(|_| Status::InternalServerError)?
        .decode()
        .map_err(|_| Status::BadRequest)?
        .write_to(&mut Cursor::new(&mut new_data_vec), ImageOutputFormat::WebP)
        .map_err(|_| Status::InternalServerError)?;

    let digest = md5::compute(&new_data_vec);
    let md5 = format!("{:x}", digest);

    let new_content_type = ContentType::WEBP;

    let objs = schema::local_files::table
        .find(&md5)
        .load::<LocalFile>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !objs.is_empty() {
        return Ok(Json(InsertResponse {
            id: objs[0].id.to_owned(),
        }));
    }

    let filename = if let Some(ext) = new_content_type.extension() {
        format!("{}.{}", md5, ext.as_str())
    } else {
        md5.to_owned()
    };

    let key = format!("{}{}", IMAGE_PREFIX, filename);

    let md5_ = md5.to_owned();

    conn.transaction::<(), TransactionError<PutObjectError>, _>(|conn| {
        /*
        let content_type = file
            .content_type()
            .or(Some(&ContentType::Binary))
            .unwrap()
            .to_owned();
        */
        let new_content_type = ContentType::WEBP;
        let length = file.len().to_owned();
        async move {
            insert_into(schema::local_files::table)
                .values((
                    schema::local_files::id.eq(md5),
                    schema::local_files::file_name.eq(&filename),
                    schema::local_files::path.eq(&key),
                ))
                .execute(conn)
                .await
                .map_err(|err| TransactionError::ResultError(err))?;

            app_state
                .s3_client
                .put_object()
                .body(ByteStream::from(new_data_vec))
                .bucket(BUCKET)
                .content_type(new_content_type.to_string())
                .content_length(length as i64)
                .key(&key)
                .send()
                .await
                .map_err(|err| TransactionError::SdkError(err))?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(transaction_error_to_status)?;

    info!("Object created: {}", md5_);

    Ok(Json(InsertResponse { id: md5_ }))
}

#[post("/image/item_from_web", data = "<url>")]
async fn create_object_from_web(
    app_state: &State<AppState>,
    auth: Option<ApiTokenClaims>,
    db: &State<db::Pool>,
    url: String,
) -> Result<Json<InsertResponse<String>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let client = reqwest::Client::new();

    let resp_head = client
        .head(&url)
        .send()
        .await
        .map_err(|_| Status::BadRequest)?
        .error_for_status()
        .map_err(|_| Status::BadRequest)?;

    let content_type_str = if let Some(v) = resp_head.headers().get("content-type") {
        v.to_str().map_or("application/octet-stream", |v| v)
    } else {
        "application/octet-stream"
    };

    let content_type = ContentType::parse_flexible(content_type_str)
        .or(Some(ContentType::Binary))
        .unwrap();

    if content_type.top().ne("image") {
        return Err(Status::UnprocessableEntity);
    };

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|_| Status::BadRequest)?
        .error_for_status()
        .map_err(|_| Status::BadRequest)?;

    let resp_data = resp.bytes().await.map_err(|_| Status::BadRequest)?;

    let mut new_data_vec: Vec<u8> = Vec::new();
    ImageReader::new(Cursor::new(resp_data))
        .with_guessed_format()
        .map_err(|_| Status::InternalServerError)?
        .decode()
        .map_err(|_| Status::BadRequest)?
        .write_to(&mut Cursor::new(&mut new_data_vec), ImageOutputFormat::WebP)
        .map_err(|_| Status::InternalServerError)?;

    let digest = md5::compute(&new_data_vec);
    let md5 = format!("{:x}", digest);

    let new_content_type = ContentType::WEBP;

    let objs = schema::local_files::table
        .find(&md5)
        .load::<LocalFile>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !objs.is_empty() {
        return Ok(Json(InsertResponse {
            id: objs[0].id.to_owned(),
        }));
    }

    let filename = if let Some(ext) = new_content_type.extension() {
        format!("{}.{}", md5, ext.as_str())
    } else {
        md5.to_owned()
    };

    let key = format!("{}{}", IMAGE_PREFIX, filename);

    let md5_ = md5.to_owned();

    conn.transaction::<(), TransactionError<PutObjectError>, _>(|conn| {
        let length = new_data_vec.len();
        async move {
            insert_into(schema::local_files::table)
                .values((
                    schema::local_files::id.eq(md5),
                    schema::local_files::file_name.eq(&filename),
                    schema::local_files::path.eq(&key),
                ))
                .execute(conn)
                .await
                .map_err(|err| TransactionError::ResultError(err))?;

            app_state
                .s3_client
                .put_object()
                .body(ByteStream::from(new_data_vec))
                .bucket(BUCKET)
                .content_type(new_content_type.to_string())
                .content_length(length as i64)
                .key(&key)
                .send()
                .await
                .map_err(|err| TransactionError::SdkError(err))?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(transaction_error_to_status)?;

    info!("Object created: {}", md5_);

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
            .map_err(result_error_to_status)?,
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
        .map_err(result_error_to_status)?;

    let id_ = id.to_owned();

    conn.transaction::<(), TransactionError<DeleteObjectError>, _>(|conn| {
        async move {
            delete(schema::local_files::table)
                .filter(schema::local_files::id.eq(id_))
                .execute(conn)
                .await
                .map_err(|err| TransactionError::ResultError(err))?;

            app_state
                .s3_client
                .delete_object()
                .bucket(BUCKET)
                .key(obj.path)
                .send()
                .await
                .map_err(|err| TransactionError::SdkError(err))?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(transaction_error_to_status)?;

    info!("Object deleted: {}", id);

    Ok(Json(DeleteResponse { id }))
}

pub fn routes() -> Vec<Route> {
    routes![
        create_object,
        create_object_from_web,
        get_object,
        delete_object
    ]
}
