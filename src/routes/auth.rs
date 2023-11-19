use crate::{db, models::*, schema};
use chrono;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rocket::{get, http::Status, post, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};
use std::env;

use diesel::{insert_into, ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: i64,
    exp: i64,
    iss: String,
}

#[post("/create_token")]
pub fn create_token(db: &State<db::Pool>, token: Option<ApiToken>) -> Result<String, Status> {
    let mut conn = db.get().unwrap();
    let token = token.ok_or(Status::Unauthorized)?;

    if token.admin {
        let now = chrono::offset::Utc::now();

        let header = Header::new(Algorithm::HS512);

        let claims = Claims {
            iat: now.timestamp(),
            iss: "uhq_blog".into(),
            exp: 4102444800, // 2100-01-01 00:00:00
        };

        let key = EncodingKey::from_secret(
            env::var("JWT_SIGNING_KEY")
                .expect("未设置JWT签名密钥")
                .as_ref(),
        );
        
        let new_token = format!(
            "Bearer {}",
            encode(&header, &claims, &key).map_err(|_| Status::InternalServerError)?
        );

        insert_into(schema::api_tokens::table)
            .values(ApiToken {
                token: new_token.clone(),
                admin: false,
                created_at: now,
            })
            .execute(&mut conn)
            .map_err(|_| Status::InternalServerError)?;

        Ok(new_token)
    } else {
        Err(Status::Forbidden)
    }
}

pub fn routes() -> Vec<Route> {
    routes![create_token]
}
