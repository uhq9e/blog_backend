extern crate diesel;

#[macro_use]
extern crate rocket;

use aws_config::{
    self, environment::credentials::EnvironmentVariableCredentialsProvider, BehaviorVersion, Region,
};
use aws_sdk_s3;
use dotenvy::dotenv;
use env_logger;
use log::info;
use rocket::data::ToByteUnit;
use std::env;

mod db;
mod models;
mod schedule_jobs;
mod schema;
mod utils;

mod routes;

pub const BUCKET: &'static str = "blog-storage";

pub struct AppState {
    pub database_url: String,
    pub jwt_signing_key: String,
    pub s3_client: aws_sdk_s3::Client,
}

pub async fn create_s3_client() -> aws_sdk_s3::Client {
    let config = aws_config::defaults(BehaviorVersion::v2023_11_09())
        .credentials_provider(EnvironmentVariableCredentialsProvider::new())
        .endpoint_url(env::var("S3_ENDPOINT_URL").expect("未设置S3_ENDPOINT_URL"))
        .region(Region::new("auto"))
        .load()
        .await;

    aws_sdk_s3::Client::new(&config)
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    dotenv().ok();

    env_logger::init();

    let client = create_s3_client().await;

    let app_state = AppState {
        database_url: env::var("DATABASE_URL").expect("未设置DATABASE_URL"),
        jwt_signing_key: env::var("JWT_SIGNING_KEY").expect("未设置JWT_SIGNING_KEY"),
        s3_client: client,
    };

    let pool = db::establish_connection(app_state.database_url.to_owned()).await;

    let limits = rocket::data::Limits::default()
        .limit("file", 20.megabytes())
        .limit("data-form", 30.megabytes());

    let config = rocket::Config {
        port: 6223,
        limits,
        ..rocket::Config::default()
    };

    schedule_jobs::init(app_state.database_url.to_owned()).await;

    rocket::custom(&config)
        .manage(pool)
        .manage(app_state)
        .mount("/api/authors", routes::authors::routes())
        .mount("/api/images", routes::images::routes())
        .mount("/api/auth", routes::auth::routes())
        .mount("/api/storage", routes::storage::routes())
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
