use crate::common::myblog_config;
use actix_web::rt::Runtime;
use autowired::bean;
use mongodb::Client;

pub const DB_NAME: &str = "myblog";

#[bean(option)]
fn build_mongodb_client() -> Option<Client> {
    let config = myblog_config();
    let client = Runtime::new()
        .unwrap()
        .block_on(Client::with_uri_str(&config.mongodb_uri));
    log::info!("build mongodb client, uri={}", config.mongodb_uri);
    client.ok()
}
