use crate::{
    db,
    models::*,
    schema,
    utils::{response::*, ApiTokenClaims, Pagination},
};
use rocket::{delete, get, http::Status, post, put, serde::json::Json, Route, State};
use serde::Deserialize;

use diesel::{
    delete, insert_into, prelude::Insertable, query_builder::AsChangeset,
    result::Error as ResultError, update, ExpressionMethods, QueryDsl,
};
use diesel_async::RunQueryDsl;

#[get("/?<pg..>")]
async fn list_authors(db: &State<db::Pool>, pg: Pagination) -> Result<Json<Vec<Author>>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;
    let authors = schema::authors::table
        .offset(pg.offset.into())
        .limit(pg.limit.into())
        .load::<Author>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(authors))
}

#[get("/<id>")]
async fn get_author(db: &State<db::Pool>, id: i32) -> Result<Json<Author>, Status> {
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::authors::table
        .find(id)
        .first::<Author>(&mut conn)
        .await
        .map_err(|err| {
            if let ResultError::NotFound = err {
                Status::NotFound
            } else {
                Status::InternalServerError
            }
        })?;

    let author = schema::authors::table
        .filter(schema::authors::id.eq(id))
        .first::<Author>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(author))
}

#[derive(Deserialize, Insertable)]
#[diesel(table_name = schema::authors)]
struct NewAuthorForm {
    name: String,
    urls: Vec<String>,
}

#[post("/", data = "<data>")]
async fn create_author(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    data: Json<NewAuthorForm>,
) -> Result<Json<InsertResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;
    let author_id = insert_into(schema::authors::table)
        .values(data.into_inner())
        .returning(schema::authors::id)
        .get_result::<i32>(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(InsertResponse { id: author_id }))
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::authors)]
struct UpdateAuthorForm {
    name: Option<String>,
    urls: Option<Vec<String>>,
}

#[put("/<id>", data = "<data>")]
async fn update_author(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
    data: Json<UpdateAuthorForm>,
) -> Result<Json<UpdateResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::authors::table
        .find(id)
        .first::<Author>(&mut conn)
        .await
        .map_err(|err| {
            if let ResultError::NotFound = err {
                Status::NotFound
            } else {
                Status::InternalServerError
            }
        })?;

    update(schema::authors::table)
        .filter(schema::authors::id.eq(id))
        .set(data.into_inner())
        .execute(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(UpdateResponse { id }))
}

#[delete("/<id>")]
async fn delete_author(
    db: &State<db::Pool>,
    auth: Option<ApiTokenClaims>,
    id: i32,
) -> Result<Json<DeleteResponse<i32>>, Status> {
    auth.ok_or(Status::Forbidden)?;
    let mut conn = db.get().await.map_err(|_| Status::InternalServerError)?;

    schema::authors::table
        .find(id)
        .first::<Author>(&mut conn)
        .await
        .map_err(|err| {
            if let ResultError::NotFound = err {
                Status::NotFound
            } else {
                Status::InternalServerError
            }
        })?;

    delete(schema::authors::table.filter(schema::authors::id.eq(id)))
        .execute(&mut conn)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(DeleteResponse { id }))
}

pub fn routes() -> Vec<Route> {
    routes![
        list_authors,
        get_author,
        create_author,
        update_author,
        delete_author
    ]
}
