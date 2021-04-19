use autowired::{Autowired, bean};
use mongodb::{Client, Collection};
use actix_web::rt::Runtime;
use crate::common::myblog_config;

pub const DB_NAME: &str = "myblog";

#[bean]
fn build_mongodb_client() -> Option<Client> {
    let config = myblog_config();
    let client = Runtime::new().unwrap().block_on(Client::with_uri_str(&config.mongodb_uri));
    log::info!("build mongodb client, uri={}", config.mongodb_uri);
    client.ok()
}

pub fn collection(coll_name: &str) -> Collection {
    Autowired::<Client>::new()
        .database(DB_NAME)
        .collection(coll_name)
}
