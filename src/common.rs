use actix_web::{HttpResponse, error};
use serde::{Serialize, Deserialize};
use failure::Fail;
use bson::Document;

#[derive(Fail, Debug)]
pub enum BusinessError {
    #[fail(display = "Validation error on field: {}", field)]
    ValidationError { field: String },
    #[fail(display = "argument error")]
    ArgumentError,
    #[fail(display = "An internal error occurred. Please try again later.")]
    InternalError,
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        let code = match *self {
            BusinessError::ValidationError { .. } => 10001,
            BusinessError::ArgumentError { .. } => 10002,
            _ => 10000,
        };
        let resp = Resp::err(code, &self.to_string());
        HttpResponse::BadRequest().json(resp)
    }
    // 重写response的序列化结果
    fn render_response(&self) -> HttpResponse {
        self.error_response()
    }
}

impl std::convert::From<bson::oid::Error> for BusinessError {
    fn from(_: bson::oid::Error) -> Self {
        BusinessError::InternalError
    }
}

impl std::convert::From<std::convert::Infallible> for BusinessError {
    fn from(_: std::convert::Infallible) -> Self {
        BusinessError::InternalError
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

impl CursorAsVec for mongodb::cursor::Cursor {
    fn as_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T> {
        self.map(|item| {
            let doc: Document = item.unwrap();
            let bson = bson::Bson::Document(doc);
            return bson::from_bson(bson).unwrap();
        }).collect()
    }
}
