use actix_web::{error, HttpResponse};
use bson::Document;
use futures::StreamExt;
use mongodb::Cursor;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use thiserror::Error;

pub type RespResult = Result<HttpResponse, BusinessError>;

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
pub struct Resp<T>
where
    T: Serialize,
{
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> Resp<T> {
    pub fn ok(data: T) -> Self {
        Resp {
            code: 0,
            message: "ok".to_owned(),
            data: Some(data),
        }
    }

    pub fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    pub fn err(error: i32, message: &str) -> Self {
        Resp {
            code: error,
            message: message.to_owned(),
            data: None,
        }
    }
}

pub trait CursorIntoVec {
    fn into_vec<T>(self) -> Pin<Box<dyn Future<Output = Vec<T>> + Unpin>>
    where
        T: 'static + DeserializeOwned;
}

impl CursorIntoVec for Cursor {
    fn into_vec<T>(self) -> Pin<Box<dyn Future<Output = Vec<T>> + Unpin>>
    where
        T: 'static + DeserializeOwned,
    {
        let fut = StreamExt::map(self, |item| {
            let doc: Document = item.unwrap();
            let bson = bson::Bson::Document(doc);
            return bson::from_bson(bson).unwrap();
        })
        .collect();
        Pin::new(Box::new(fut))
    }
}

pub fn struct_into_document<'a, T: Sized + Serialize + Deserialize<'a>>(t: &T) -> Option<Document> {
    let mid: Option<Document> = bson::to_bson(t)
        .ok()
        .map(|x| x.as_document().unwrap().to_owned());

    mid.map(|mut doc| {
        let keys = doc.keys();
        let rm: Vec<String> = keys
            .filter(|k| doc.is_null(k))
            .map(|x| x.to_owned())
            .collect();
        // remove null value fields
        for x in rm {
            doc.remove(&x);
        }
        doc
    })
}

pub fn init_logger() {
    use chrono::Local;
    use std::io::Write;

    let env = env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    // 设置日志打印格式
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                buf.default_styled_level(record.level()),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();
    log::info!("env_logger initialized.");
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlogConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_mongodb_uri")]
    pub mongodb_uri: String,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8000
}

fn default_mongodb_uri() -> String {
    "mongodb://localhost:27017".to_string()
}

pub fn load_config(path: impl AsRef<Path>) -> anyhow::Result<BlogConfig> {
    let text = std::fs::read_to_string(path)?;
    let config = serde_yaml::from_str(&text)?;
    Ok(config)
}
