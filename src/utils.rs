use crate::AppState;
use aws_sdk_s3::{error::SdkError, primitives::SdkBody};
use aws_smithy_runtime_api::http::Response;
use diesel_order_with_direction::QueryOrderDirection;
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
    #[field(default = 0, validate = range(0..))]
    pub offset: i64,
    #[field(default = 20, validate = range(0..101))]
    pub limit: i64,
    #[field(default = 1, validate = range(-1..2))]
    pub order: i8,
    #[field(default = "+id")]
    pub order_by: String,
}

#[derive(Debug)]
pub enum ApiTokenError {
    MissingHeader,
    ValidationError,
    FormatError,
}

pub enum TransactionError<T> {
    ResultError(diesel::result::Error),
    SdkError(SdkError<T, Response<SdkBody>>),
}

pub fn sdk_error_to_status<T>(err: SdkError<T, Response<SdkBody>>) -> Status {
    if let SdkError::ServiceError(_) = err {
        Status::FailedDependency
    } else {
        Status::BadGateway
    }
}

pub fn result_error_to_status(err: diesel::result::Error) -> Status {
    if let diesel::result::Error::NotFound = err {
        Status::NotFound
    } else {
        Status::InternalServerError
    }
}

pub fn result_error_to_status_failed_dependency(err: diesel::result::Error) -> Status {
    if let diesel::result::Error::NotFound = err {
        Status::FailedDependency
    } else {
        Status::InternalServerError
    }
}

pub fn transaction_error_to_status<T>(err: TransactionError<T>) -> Status {
    match err {
        TransactionError::SdkError(err) => sdk_error_to_status(err),
        TransactionError::ResultError(err) => result_error_to_status(err),
    }
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

pub struct ParsedOrderBy {
    pub column: String,
    pub direction: QueryOrderDirection,
}

pub fn parse_order_from_string(str: String) -> Vec<Option<ParsedOrderBy>> {
    let re = Regex::new(r"^(?P<dir>[+-]?)(?P<column>\w{1,25})$").unwrap();

    let mut parsed_items: Vec<Option<ParsedOrderBy>> = Vec::new();
    for str in str.split(",").map(|v| v.trim()) {
        parsed_items.push(if let Some(cap) = re.captures(str) {
            Some(ParsedOrderBy {
                column: cap["column"].to_string(),
                direction: match &cap["dir"] {
                    "+" => QueryOrderDirection::Ascending,
                    "-" => QueryOrderDirection::Descending,
                    _ => QueryOrderDirection::Ascending,
                },
            })
        } else {
            None
        })
    }

    parsed_items
}

pub mod response {
    use serde::Serialize;

    #[derive(Serialize, Debug, Clone)]
    pub struct InsertResponse<T> {
        pub id: T,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct UpdateResponse<T> {
        pub id: T,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct DeleteResponse<T> {
        pub id: T,
    }

    #[derive(Serialize, Debug, Clone)]
    pub struct ListResponse<T> {
        pub items: Vec<T>,
        pub count: Option<i64>,
    }

    impl<T> ListResponse<T> {
        pub fn new(items: Vec<T>) -> Self {
            Self { items, count: None }
        }

        pub fn count(mut self, count: i64) -> Self {
            self.count = Some(count);
            self
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

pub mod naive_date_format_option {
    use chrono::NaiveDate;
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &'static str = "%Y-%m-%d";

    pub fn serialize<S>(date: &Option<NaiveDate>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if date.is_none() {
            return serializer.serialize_none();
        };
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
