use super::Article;
use crate::article::ArticleQuery;
use crate::common::*;
use actix_web::{web, HttpRequest, HttpResponse};
use bson::oid::ObjectId;
use bson::Document;
use log::*;
use serde::{Deserialize, Serialize};
use crate::middleware::mongodb::{collection};

type SimpleResp = Result<HttpResponse, BusinessError>;

fn struct_to_document<'a, T: Sized + Serialize + Deserialize<'a>>(t: &T) -> Option<Document> {
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

pub async fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();
    let d: Document = struct_to_document(&article).unwrap();

    let rs = collection(Article::TABLE_NAME).insert_one(d, None).await?;
    let new_id: String = rs.inserted_id.as_object_id().map(ObjectId::to_hex).unwrap();
    info!("save article, id={}", new_id);
    Resp::ok(new_id).to_json_result()
}

pub async fn list_article(query: web::Json<ArticleQuery>) -> SimpleResp {
    let query = query.into_inner();

    // 构造查询参数
    let mut d: Document = doc! {};
    if query._id.is_some() {
        d.insert("_id", query._id.unwrap());
    }

    if !query.keyword.is_empty() {
        d.insert(
            "$or",
            bson::Bson::Array(vec![
                doc! {"title": {"$regex": & query.keyword, "$options": "i"}}.into(),
                doc! {"author": {"$regex": & query.keyword, "$options": "i"}}.into(),
                doc! {"content": {"$regex": & query.keyword, "$options": "i"}}.into(),
            ]),
        );
    }

    let coll = collection("article");
    match coll.find(Some(d), None).await {
        Ok(cursor) => {
            let list = cursor.into_vec::<Article>().await;
            Resp::ok(list).to_json_result()
        }
        Err(e) => {
            error!("list_article error, {:?}", e);
            return Err(BusinessError::InternalError { source: anyhow!(e) });
        }
    }
}

pub async fn update_article(req: HttpRequest, article: web::Json<Article>) -> SimpleResp {
    let id = req.match_info().get("id").unwrap_or("");

    let oid = ObjectId::with_string(id).map_err(|e| {
        log::error!("update_article, can't parse id to ObjectId, {:?}", e);
        BusinessError::ValidationError {
            field: "id".to_owned(),
        }
    })?;

    let article = article.into_inner();

    let filter = doc! {"_id": oid};

    let update = doc! {"$set": struct_to_document( & article).unwrap()};

    let effect = match collection(Article::TABLE_NAME)
        .update_one(filter, update, None)
        .await
    {
        Ok(result) => {
            info!(
                "update article, id={}, effect={}",
                id, result.modified_count
            );
            result.modified_count
        }
        Err(e) => {
            error!("update_article, failed to visit db, id={}, {:?}", id, e);
            return Err(BusinessError::InternalError { source: anyhow!(e) });
        }
    };

    Resp::ok(effect).to_json_result()
}

pub async fn remove_article(req: HttpRequest) -> SimpleResp {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError {
            field: "id".to_owned(),
        });
    }

    let filter = doc! {"_id": ObjectId::with_string(id).unwrap()};

    let effect = match collection(Article::TABLE_NAME).delete_one(filter, None).await {
        Ok(result) => {
            info!("delete article, id={}, effect={}", id, result.deleted_count);
            result.deleted_count
        }
        Err(e) => {
            error!("remove_article, failed to visit db, id={}, {:?}", id, e);
            return Err(BusinessError::InternalError { source: anyhow!(e) });
        }
    };

    Resp::ok(effect).to_json_result()
}
