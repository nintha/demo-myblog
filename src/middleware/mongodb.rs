use autowired::{Autowired, bean};
use mongodb::{Client, Collection};
use actix_web::rt::Runtime;

#[bean]
fn build_mongodb_client() -> Option<Client> {
    let client = Runtime::new().unwrap().block_on(Client::with_uri_str("mongodb://localhost:27017"));
    log::info!("build mongodb client");
    client.ok()
}

pub fn collection(coll_name: &str) -> Collection {
    Autowired::<Client>::new()
        .database("myblog")
        .collection(coll_name)
}
