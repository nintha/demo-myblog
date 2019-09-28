mod handler;

use serde::{Serialize, Deserialize, Serializer};
pub use handler::*;
use bson::oid::ObjectId;

#[derive(Deserialize, Serialize, Debug)]
pub struct Article {
    #[serde(serialize_with = "serialize_object_id")]
    _id: Option<ObjectId>,
    title: String,
    author: String,
    content: String,
}

impl Article {
    pub const TABLE_NAME: &'static str = "article";
}

pub fn serialize_object_id<S>(oid: &Option<ObjectId>, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
    match oid.as_ref().map(|x| x.to_hex()) {
        Some(v) => s.serialize_str(&v),
        None => s.serialize_none()
    }
}