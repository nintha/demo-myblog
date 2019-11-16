use super::Article;
use actix_web::{HttpResponse, web, HttpRequest};
use crate::collection;
use log::*;
use bson::oid::ObjectId;
use bson::Document;
use bson::ordered::OrderedDocument;
use crate::common::*;
use serde::{Deserialize, Serialize};
use mongodb::doc;

type SimpleResp = Result<HttpResponse, BusinessError>;

pub fn struct_to_document<'a, T: Sized + Serialize + Deserialize<'a>>(t: &T) -> Option<OrderedDocument> {
    let mid: Option<OrderedDocument> = bson::to_bson(t)
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

pub fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();
    let d: Document = struct_to_document(&article).unwrap();

    let result = collection(Article::TABLE_NAME).insert_one(d, None);
    match result {
        Ok(rs) => {
            let new_id: String = rs.inserted_id
                .and_then(|x| x.as_object_id().map(ObjectId::to_hex))
                .ok_or_else(|| {
                    error!("save_article error, can not get inserted id");
                    BusinessError::InternalError
                })?;
            info!("save article, id={}", new_id);
            Resp::ok(new_id).to_json_result()
        }
        Err(e) => {
            error!("save_article error, {}", e);
            Err(BusinessError::InternalError)
        }
    }
}

pub fn list_article() -> SimpleResp {
    let coll = collection("article");

    let cursor = coll.find(Some(doc! {}), None);
    let result = cursor.map(|mut x| x.as_vec::<Article>());
    match result {
        Ok(list) => Resp::ok(list).to_json_result(),
        Err(e) => {
            error!("list_article error, {}", e);
            return Err(BusinessError::InternalError);
        }
    }
}

pub fn update_article(req: HttpRequest, article: web::Json<Article>) -> SimpleResp {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError { field: "id".to_owned() });
    }
    let article = article.into_inner();

    let filter = doc! {"_id" => ObjectId::with_string(id)?};

    let update = doc! {"$set": struct_to_document(&article).unwrap()};

    let effect = match collection(Article::TABLE_NAME).update_one(filter, update, None) {
        Ok(result) => {
            info!("update article, id={}, effect={}", id, result.modified_count);
            result.modified_count
        }
        Err(e) => {
            error!("update_article, failed to visit db, id={}, {}", id, e);
            return Err(BusinessError::InternalError);
        }
    };

    Resp::ok(effect).to_json_result()
}

pub fn remove_article(req: HttpRequest) -> SimpleResp {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError { field: "id".to_owned() });
    }

    let filter = doc! {"_id" => ObjectId::with_string(id).unwrap()};

    let effect = match collection(Article::TABLE_NAME).delete_one(filter, None) {
        Ok(result) => {
            info!("delete article, id={}, effect={}", id, result.deleted_count);
            result.deleted_count
        }
        Err(e) => {
            error!("remove_article, failed to visit db, id={}, {}", id, e);
            return Err(BusinessError::InternalError);
        }
    };

    Resp::ok(effect).to_json_result()
}