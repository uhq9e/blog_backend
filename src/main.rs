extern crate diesel;

#[macro_use]
extern crate rocket;

use dotenvy::dotenv;

mod db;
mod models;
mod schema;
mod utils;

mod routes;

#[launch]
fn rocket() -> _ {
    dotenv().ok();
    let pool = db::establish_connection();

    rocket::build()
        .manage(pool)
        .mount("/api/authors", routes::authors::routes())
        .mount("/api/images", routes::images::routes())
        .mount("/api/auth", routes::auth::routes())
}
