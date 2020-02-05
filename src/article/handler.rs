use super::Article;
use actix_web::{HttpResponse, web, HttpRequest};
use crate::collection;
use log::*;
use bson::oid::ObjectId;
use bson::Document;
use bson::ordered::OrderedDocument;
use crate::common::*;
use serde::{Deserialize, Serialize};
use crate::article::ArticleQuery;

type SimpleResp = Result<HttpResponse, BusinessError>;

fn struct_to_document<'a, T: Sized + Serialize + Deserialize<'a>>(t: &T) -> Option<OrderedDocument> {
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

pub async fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();
    let d: Document = struct_to_document(&article).unwrap();

    let result = collection(Article::TABLE_NAME).insert_one(d, None);
    match result {
        Ok(rs) => {
            let new_id: String = rs.inserted_id
                .as_object_id()
                .map(ObjectId::to_hex)
                .ok_or_else(|| {
                    BusinessError::InternalError { from: "save_article error, can not get inserted id".to_owned() }
                })?;
            info!("save article, id={}", new_id);
            Resp::ok(new_id).to_json_result()
        }
        Err(e) => {
            error!("save_article error, {}", e);
            Err(BusinessError::InternalError { from: "save_article error".to_owned() })
        }
    }
}

pub async fn list_article(query: web::Json<ArticleQuery>) -> SimpleResp {
    let query = query.into_inner();

    // 构造查询参数
    let mut d: Document = doc! {};
    if query._id.is_some() {
        d.insert("_id", query._id.unwrap());
    }

    if !query.keyword.is_empty() {
        d.insert("$or", bson::Bson::Array(vec![
            doc! {"title": {"$regex": &query.keyword, "$options": "i"}}.into(),
            doc! {"author": {"$regex": &query.keyword, "$options": "i"}}.into(),
            doc! {"content": {"$regex": &query.keyword, "$options": "i"}}.into(),
        ]));
    }

    let coll = collection("article");
    let cursor = coll.find(Some(d), None);
    let result = cursor.map(|mut x| x.as_vec::<Article>());
    match result {
        Ok(list) => Resp::ok(list).to_json_result(),
        Err(e) => {
            error!("list_article error, {}", e);
            return Err(BusinessError::InternalError { from: "list_article error".to_owned() });
        }
    }
}

pub async fn update_article(req: HttpRequest, article: web::Json<Article>) -> SimpleResp {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError { field: "id".to_owned() });
    }
    let article = article.into_inner();

    let oid = ObjectId::with_string(id)?;

    let filter = doc! {"_id" => oid};

    let update = doc! {"$set": struct_to_document(&article).unwrap()};

    let effect = match collection(Article::TABLE_NAME).update_one(filter, update, None) {
        Ok(result) => {
            info!("update article, id={}, effect={}", id, result.modified_count);
            result.modified_count
        }
        Err(e) => {
            error!("update_article, failed to visit db, id={}, {}", id, e);
            return Err(BusinessError::InternalError { from: "update_article, failed to visit db".to_owned() });
        }
    };

    Resp::ok(effect).to_json_result()
}

pub async fn remove_article(req: HttpRequest) -> SimpleResp {
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
            return Err(BusinessError::InternalError { from: "remove_article, failed to visit db".to_owned() });
        }
    };

    Resp::ok(effect).to_json_result()
}