use aws_sdk_s3::{
    operation::delete_object::DeleteObjectError, operation::put_object::PutObjectError,
    primitives::ByteStream,
};
use diesel::{delete, insert_into, ExpressionMethods, QueryDsl};
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};
use log::info;
use md5;
use rocket::{
    fs::TempFile,
    // get,
    http::{ContentType, Status},
    post,
    serde::json::Json,
    tokio::io,
    Route,
    State,
};
use uuid::Uuid;

use crate::{
    db,
    misc::enums::SiteContentKind,
    models::*,
    schema,
    utils::{
        response::{DeleteResponse, InsertResponse},
        result_error_to_status, transaction_error_to_status, ApiTokenClaims, TransactionError,
    },
    AppState, BUCKET,
};

const NOVEL_PREFIX: &'static str = "novel/";

#[post("/novel/item", data = "<file>")]
pub async fn create_novel_object(
    app_state: &State<AppState>,
    auth: Option<ApiTokenClaims>,
    db: &State<db::Pool>,
    file: TempFile<'_>,
) -> Result<Json<InsertResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let binary = ContentType::Binary;
    let content_type = file.content_type().or(Some(&binary)).unwrap();

    if content_type.ne(&ContentType::PDF) {
        return Err(Status::UnprocessableEntity);
    };

    let mut data_stream = file.open().await.map_err(|_| Status::BadRequest)?;
    let mut data_vec: Vec<u8> = vec![];
    io::copy(&mut data_stream, &mut data_vec)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let digest = md5::compute(&data_vec);
    let md5 = format!("{:x}", digest);

    let objs = schema::site_storage::table
        .filter(schema::site_storage::hash.eq(&md5))
        .filter(schema::site_storage::kind.eq(SiteContentKind::Novel as i16))
        .load::<SiteStorage>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if !objs.is_empty() {
        return Ok(Json(InsertResponse {
            id: objs[0].id.to_owned(),
        }));
    };

    let filename = format!(
        "{}.{}",
        Uuid::new_v4().to_string(),
        content_type.extension().unwrap().as_str()
    );

    let key = format!("{}{}", NOVEL_PREFIX, filename);

    let inserted_id = conn
        .transaction::<i32, TransactionError<PutObjectError>, _>(|conn| {
            /*
            let content_type = file
                .content_type()
                .or(Some(&ContentType::Binary))
                .unwrap()
                .to_owned();
            */
            let new_content_type = ContentType::PDF;
            let length = file.len().to_owned();
            async move {
                let id = insert_into(schema::site_storage::table)
                    .values((
                        schema::site_storage::file_name.eq(&filename),
                        schema::site_storage::key.eq(&key),
                        schema::site_storage::size.eq(&(length as i64)),
                        schema::site_storage::hash.eq(&md5),
                        schema::site_storage::kind.eq(SiteContentKind::Novel as i16),
                        schema::site_storage::mime_type.eq(&new_content_type.to_string()),
                    ))
                    .returning(schema::site_storage::id)
                    .get_result::<i32>(conn)
                    .await
                    .map_err(|err| TransactionError::ResultError(err))?;

                app_state
                    .s3_client
                    .put_object()
                    .body(ByteStream::from(data_vec))
                    .bucket(BUCKET)
                    .content_type(new_content_type.to_string())
                    .content_length(length as i64)
                    .key(&key)
                    .send()
                    .await
                    .map_err(|err| TransactionError::SdkError(err))?;

                Ok(id)
            }
            .scope_boxed()
        })
        .await
        .map_err(transaction_error_to_status)?;

    info!("Novel object created: {}", inserted_id);

    Ok(Json(InsertResponse { id: inserted_id }))
}

#[delete("/novel/item/<id>")]
pub async fn delete_novel_object(
    app_state: &State<AppState>,
    auth: Option<ApiTokenClaims>,
    db: &State<db::Pool>,
    id: i32,
) -> Result<Json<DeleteResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    let obj = schema::site_storage::table
        .find(id)
        .first::<SiteStorage>(&mut conn)
        .await
        .map_err(result_error_to_status)?;

    conn.transaction::<(), TransactionError<DeleteObjectError>, _>(|conn| {
        async move {
            delete(schema::site_storage::table.filter(schema::site_storage::id.eq(id)))
                .execute(conn)
                .await
                .map_err(|err| TransactionError::ResultError(err))?;

            app_state
                .s3_client
                .delete_object()
                .bucket(BUCKET)
                .key(obj.key)
                .send()
                .await
                .map_err(|err| TransactionError::SdkError(err))?;

            Ok(())
        }
        .scope_boxed()
    })
    .await
    .map_err(transaction_error_to_status)?;

    Ok(Json(DeleteResponse { id: id }))
}

pub fn routes() -> Vec<Route> {
    routes![create_novel_object, delete_novel_object]
}
