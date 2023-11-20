extern crate diesel;

#[macro_use]
extern crate rocket;

use dotenvy::dotenv;
use std::env;

mod db;
mod models;
mod schema;
mod utils;

mod routes;

pub struct AppState {
    pub database_url: String,
    pub jwt_signing_key: String,
}

#[launch]
fn rocket() -> _ {
    dotenv().ok();

    let app_state = AppState {
        database_url: env::var("DATABASE_URL").expect("未设置DATABASE_URL"),
        jwt_signing_key: env::var("JWT_SIGNING_KEY").expect("未设置JWT_SIGNING_KEY")
    };

    let pool = db::establish_connection(app_state.database_url.to_owned());

    rocket::build()
        .manage(pool)
        .manage(app_state)
        .mount("/api/authors", routes::authors::routes())
        .mount("/api/images", routes::images::routes())
        .mount("/api/auth", routes::auth::routes())
}
