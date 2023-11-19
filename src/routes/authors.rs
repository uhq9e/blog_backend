use crate::{db, models::*, schema, utils::Pagination};
use rocket::{get, http::Status, serde::json::Json, Route, State};

use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

#[get("/?<pg..>")]
pub fn list_authors(
    db: &State<db::Pool>,
    pg: Pagination,
) -> Result<Json<Vec<Author>>, Status> {
    let mut conn = db.get().unwrap();
    let authors = schema::authors::table
        .offset(pg.offset.into())
        .limit(pg.limit.into())
        .load::<Author>(&mut conn)
        .map_err(|_| Status::InternalServerError)?;

    Ok(Json(authors))
}

#[get("/<id>")]
pub fn get_author(db: &State<db::Pool>, id: i32) -> Result<Json<Author>, Status> {
    let mut conn = db.get().unwrap();
    let author = schema::authors::table
        .filter(schema::authors::id.eq(id))
        .first::<Author>(&mut conn)
        .map_err(|_| Status::NotFound)?;

    Ok(Json(author))
}

pub fn routes() -> Vec<Route> {
    routes![list_authors, get_author]
}
