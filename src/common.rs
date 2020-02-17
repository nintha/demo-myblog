use actix_web::{HttpResponse, error};
use serde::{Serialize, Deserialize};
use bson::Document;
use mongodb::Cursor;
use thiserror::Error;

/// error format "code#message"
#[derive(Error, Debug)]
pub enum BusinessError {
    #[error("10001#Validation error on field: {field}")]
    ValidationError { field: String },
    #[error("10002#argument error")]
    ArgumentError,
    #[error("10000#An internal error occurred. Please try again later.")]
    InternalError {
        #[source]
        source: anyhow::Error,
    },
}

impl BusinessError {
    fn to_code(&self) -> i32 {
        let code = &self.to_string()[0..5];
        code.parse().unwrap_or(-1)
    }

    fn to_message(&self) -> String {
        self.to_string()[6..].to_owned()
    }
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        let resp = Resp::err(self.to_code(), &self.to_message());
        HttpResponse::BadRequest().json(resp)
    }
}

impl From<mongodb::error::Error> for BusinessError {
    fn from(e: mongodb::error::Error) -> Self {
        log::error!("mongodb error, {}", e.to_string());
        BusinessError::InternalError { source: anyhow!(e) }
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resp<T> where T: Serialize {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> Resp<T> {
    pub fn ok(data: T) -> Self {
        Resp { code: 0, message: "ok".to_owned(), data: Some(data) }
    }

    pub fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    pub fn err(error: i32, message: &str) -> Self {
        Resp { code: error, message: message.to_owned(), data: None }
    }
}

pub trait CursorAsVec {
    fn as_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T>;
}

impl CursorAsVec for Cursor {
    fn as_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T> {
        self.map(|item| {
            let doc: Document = item.unwrap();
            let bson = bson::Bson::Document(doc);
            return bson::from_bson(bson).unwrap();
        }).collect()
    }
}
