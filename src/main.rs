use log::info;
use lazy_static::lazy_static;
use mongodb::{Client, ThreadedClient};
use mongodb::db::ThreadedDatabase;
use mongodb::coll::Collection;
use actix_web::{web, App, HttpServer, FromRequest, error, HttpResponse};
use crate::article::Article;
use crate::common::Resp;

mod common;
mod article;

lazy_static! {
    pub static ref MONGO: Client = create_mongo_client();
}

fn create_mongo_client() -> Client {
    Client::connect("localhost", 27017)
        .expect("Failed to initialize standalone client.")
}

fn collection(coll_name: &str) -> Collection {
    MONGO.db("myblog").collection(coll_name)
}

fn init_logger() {
    use chrono::Local;
    use std::io::Write;

    let env = env_logger::Env::default()
        .filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
    // 设置日志打印格式
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();
    info!("env_logger initialized.");
}


fn main() {
    init_logger();

    let binding_address = "0.0.0.0:8000";
    let server = HttpServer::new(|| App::new()
        .data(// change json extractor configuration
              web::Json::<Article>::configure(|cfg| {
                  cfg.error_handler(|err, _req| {
                      // <- create custom error response
                      log::error!("json extractor error, path={}, {}", _req.uri(), err);
                      let resp = Resp::err(10002, "argument error");
                      error::InternalError::from_response(
                          err,
                          HttpResponse::BadRequest().json(resp),
                      ).into()
                  })
              })
        )
        .service(
            web::scope("/articles")
                .route("", web::get().to(article::list_article))
                .route("", web::post().to(article::save_article))
                .route("{id}", web::put().to(article::update_article))
                .route("{id}", web::delete().to(article::remove_article))
        ))
        .bind(binding_address)
        .expect("Can not bind to port 8000");

    server.run().unwrap();
}