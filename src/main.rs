#[macro_use]
extern crate bson;
#[macro_use]
extern crate anyhow;

use crate::article::Article;
use crate::common::*;
use actix_web::{web, App, FromRequest, HttpServer};
use log::info;

mod article;
mod common;
mod middleware;

fn init_logger() {
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
    info!("env_logger initialized.");
}
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    init_logger();
    actix_web::web::block(|| Result::<(), ()>::Ok(autowired::setup_submitted_beans())).await?;

    let binding_address = "0.0.0.0:8000";
    HttpServer::new(|| {
        App::new()
            .app_data(web::Json::<Article>::configure(|cfg| {
                cfg.error_handler(|err, req| {
                    log::error!("json extractor error, path={}, {}", req.uri(), err);
                    BusinessError::ArgumentError.into()
                })
            }))
            .service(
                web::scope("/articles")
                    .route("", web::get().to(article::list_article))
                    .route("", web::post().to(article::save_article))
                    .route("{id}", web::put().to(article::update_article))
                    .route("{id}", web::delete().to(article::remove_article)),
            )
    })
    .bind(binding_address)
    .expect(&format!("Can not bind to {}", binding_address))
    .run()
    .await?;
    Ok(())
}
