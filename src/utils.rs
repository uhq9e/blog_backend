use crate::AppState;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use regex::Regex;
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use serde::{Deserialize, Serialize};

#[derive(FromForm, Debug)]
pub struct Pagination {
    #[field(default = 0)]
    pub offset: u32,
    #[field(default = 20, validate = range(..100))]
    pub limit: u32,
}

#[derive(Debug)]
pub enum ApiTokenError {
    MissingHeader,
    ValidationError,
    FormatError,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiTokenClaims {
    pub iat: i64,
    pub exp: i64,
    pub iss: String,
    pub admin: bool,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiTokenClaims {
    type Error = ApiTokenError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("Authorization") {
            let signing_key = request
                .rocket()
                .state::<AppState>()
                .unwrap()
                .jwt_signing_key
                .to_owned();
            let re = Regex::new(r"^Bearer (?P<token>\S+)$").unwrap();

            if let Some(caps) = re.captures(token) {
                if let Ok(token_decoded) = decode::<ApiTokenClaims>(
                    &caps["token"],
                    &DecodingKey::from_secret(signing_key.as_ref()),
                    &Validation::new(Algorithm::HS512),
                ) {
                    Outcome::Success(token_decoded.claims)
                } else {
                    Outcome::Error((Status::Ok, ApiTokenError::ValidationError))
                }
            } else {
                Outcome::Error((Status::Ok, ApiTokenError::FormatError))
            }
        } else {
            Outcome::Error((Status::Ok, ApiTokenError::MissingHeader))
        }
    }
}

pub mod response {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct InsertResponse<T> {
        pub id: T,
    }

    #[derive(Serialize)]
    pub struct UpdateResponse<T> {
        pub id: T,
    }

    #[derive(Serialize)]
    pub struct DeleteResponse<T> {
        pub id: T,
    }
}

pub mod datetime_format {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.to_rfc3339());
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(DateTime::parse_from_rfc3339(s.as_str())
            .map_err(serde::de::Error::custom)?
            .into())
    }
}

pub mod naive_date_format {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn serialize<S>(date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)
    }
}

pub mod naive_date_format_option {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn serialize<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if date.is_none() {return serializer.serialize_none()};
        let date = date.unwrap();

        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<NaiveDate>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let date = NaiveDate::parse_from_str(&s, FORMAT).map_err(serde::de::Error::custom)?;
        Ok(Some(date))
    }
}
