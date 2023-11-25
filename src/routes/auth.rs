use crate::{
    utils::{ApiTokenClaims, ApiTokenError},
    AppState,
};
use chrono;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rocket::{http::Status, post, Route, State};

#[post("/create_token")]
pub fn create_token(
    state: &State<AppState>,
    auth: Result<ApiTokenClaims, ApiTokenError>,
) -> Result<String, Status> {
    let auth = auth.map_err(|_| Status::Unauthorized)?;

    if auth.admin {
        let now = chrono::offset::Utc::now();

        let header = Header::new(Algorithm::HS512);

        let claims = ApiTokenClaims {
            iat: now.timestamp(),
            iss: "uhq_blog".into(),
            exp: now.timestamp() + 3153600000, // 100年后
            admin: false,
        };

        let key = EncodingKey::from_secret(state.jwt_signing_key.as_ref());

        let new_token = format!(
            "Bearer {}",
            encode(&header, &claims, &key).map_err(|_| Status::InternalServerError)?
        );

        Ok(new_token)
    } else {
        Err(Status::Forbidden)
    }
}

pub fn routes() -> Vec<Route> {
    routes![create_token]
}
