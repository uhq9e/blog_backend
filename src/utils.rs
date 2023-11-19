use crate::{db, models::ApiToken, schema};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

use diesel::{QueryDsl, RunQueryDsl};

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
    MissingRecord,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiToken {
    type Error = ApiTokenError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        if let Some(token) = request.headers().get_one("Authorization") {
            let mut conn = request.rocket().state::<db::Pool>().unwrap().get().unwrap();

            if let Ok(model) = schema::api_tokens::table
                .find(token)
                .first::<ApiToken>(&mut conn)
            {
                Outcome::Success(model)
            } else {
                Outcome::Error((Status::Ok, ApiTokenError::MissingRecord))
            }
        } else {
            Outcome::Error((Status::Ok, ApiTokenError::MissingHeader))
        }
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
