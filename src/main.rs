extern crate diesel;

#[macro_use]
extern crate rocket;

use dotenvy::dotenv;
use std::env;
use aws_config::{self, BehaviorVersion, Region, environment::credentials::EnvironmentVariableCredentialsProvider};
use aws_sdk_s3;

mod db;
mod models;
mod schema;
mod utils;

mod routes;

pub const BUCKET: &'static str = "blog-storage";

pub struct AppState {
    pub database_url: String,
    pub jwt_signing_key: String,
    pub s3_client: aws_sdk_s3::Client,
}

#[launch]
async fn rocket() -> _ {
    dotenv().ok();

    let config = aws_config::defaults(BehaviorVersion::v2023_11_09())
        .credentials_provider(EnvironmentVariableCredentialsProvider::new())
        .endpoint_url(env::var("S3_ENDPOINT_URL").expect("未设置S3_ENDPOINT_URL"))
        .region(Region::new("auto"))
        .load().await;

    let client = aws_sdk_s3::Client::new(&config);

    let app_state = AppState {
        database_url: env::var("DATABASE_URL").expect("未设置DATABASE_URL"),
        jwt_signing_key: env::var("JWT_SIGNING_KEY").expect("未设置JWT_SIGNING_KEY"),
        s3_client: client,
    };

    let pool = db::establish_connection(app_state.database_url.to_owned()).await;

    let config = rocket::Config {
        port: 6223,
        ..rocket::Config::default()
    };

    rocket::custom(&config)
        .manage(pool)
        .manage(app_state)
        .mount("/api/authors", routes::authors::routes())
        .mount("/api/images", routes::images::routes())
        .mount("/api/auth", routes::auth::routes())
        .mount("/api/storage", routes::storage::routes())
}
