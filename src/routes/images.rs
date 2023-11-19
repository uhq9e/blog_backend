use crate::{db, models::*, schema, utils::Pagination};
use rocket::{get, http::Status, serde::json::Json, Route, State};
use serde::Serialize;

use diesel::{
    ExpressionMethods, QueryDsl, RunQueryDsl,
};

#[derive(Serialize)]
pub struct ImageItemFull {
    #[serde(flatten)]
    image_item: ImageItem,
    author: Option<Author>,
    social_post: SocialPost,
    local_file: LocalFile,
}

#[get("/?<pg..>")]
pub fn list_image_items(
    db: &State<db::Pool>,
    pg: Pagination,
) -> Result<Json<Vec<ImageItemFull>>, Status> {
    let mut conn = db.get().unwrap();

    let items_authors: Vec<(ImageItem, Option<Author>, SocialPost, LocalFile)> =
        schema::image_items::table
            .offset(pg.offset.into())
            .limit(pg.limit.into())
            .left_join(schema::authors::table)
            .inner_join(schema::social_posts::table)
            .inner_join(schema::local_files::table)
            .load::<(ImageItem, Option<Author>, SocialPost, LocalFile)>(&mut conn)
            .map_err(|_| Status::InternalServerError)?;

    let results = items_authors
        .iter()
        .map(
            |(image_item, author, social_post, local_file)| ImageItemFull {
                image_item: image_item.to_owned(),
                author: author.to_owned(),
                social_post: social_post.to_owned(),
                local_file: local_file.to_owned(),
            },
        )
        .collect::<Vec<ImageItemFull>>();

    Ok(Json(results))
}

pub fn routes() -> Vec<Route> {
    routes![list_image_items]
}
