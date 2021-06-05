use actix_web::{web, HttpRequest};
use autowired::Autowired;
use bson::oid::ObjectId;
use bson::Document;

use crate::article::service::ArticleService;
use crate::article::ArticleQuery;
use crate::common::*;
use crate::middleware::mongodb::collection;

use super::Article;
use crate::common::service::MongodbCrudService;

const ARTICLE_SERVICE: Autowired<ArticleService> = Autowired::new();

pub async fn save_article(article: web::Json<Article>) -> RespResult {
    let article: Article = article.into_inner();
    let id = ARTICLE_SERVICE.save(&article).await?;
    log::info!("save_article, id={}", id);
    Resp::ok(id).to_json_result()
}

pub async fn list_article(query: web::Json<ArticleQuery>) -> RespResult {
    let query = query.into_inner();

    // 构造查询参数
    let mut filter: Document = doc! {};
    if query._id.is_some() {
        filter.insert("_id", query._id.unwrap());
    }

    // 关键字模糊查询
    if !query.keyword.is_empty() {
        filter.insert(
            "$or",
            bson::Bson::Array(vec![
                doc! {"title": {"$regex": & query.keyword, "$options": "i"}}.into(),
                doc! {"author": {"$regex": & query.keyword, "$options": "i"}}.into(),
                doc! {"content": {"$regex": & query.keyword, "$options": "i"}}.into(),
            ]),
        );
    }

    let list = ARTICLE_SERVICE.list_with_filter(filter).await?;
    Resp::ok(list).to_json_result()
}

pub async fn update_article(req: HttpRequest, article: web::Json<Article>) -> RespResult {
    let id = req.match_info().get("id").unwrap_or("");

    let oid = ObjectId::with_string(id).map_err(|e| {
        log::error!("update_article, can't parse id to ObjectId, {:?}", e);
        BusinessError::ValidationError {
            field: "id".to_owned(),
        }
    })?;

    let article = article.into_inner();

    let filter = doc! {"_id": oid};

    let update = doc! {"$set": struct_into_document( & article).unwrap()};

    let effect = match collection(Article::TABLE_NAME)
        .update_one(filter, update, None)
        .await
    {
        Ok(result) => {
            log::info!(
                "update article, id={}, effect={}",
                id,
                result.modified_count
            );
            result.modified_count
        }
        Err(e) => {
            log::error!("update_article, failed to visit db, id={}, {:?}", id, e);
            return Err(BusinessError::InternalError { source: anyhow!(e) });
        }
    };

    Resp::ok(effect).to_json_result()
}

pub async fn remove_article(req: HttpRequest) -> RespResult {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError {
            field: "id".to_owned(),
        });
    }

    let filter = doc! {"_id": ObjectId::with_string(id).unwrap()};

    let effect = match collection(Article::TABLE_NAME)
        .delete_one(filter, None)
        .await
    {
        Ok(result) => {
            log::info!("delete article, id={}, effect={}", id, result.deleted_count);
            result.deleted_count
        }
        Err(e) => {
            log::error!("remove_article, failed to visit db, id={}, {:?}", id, e);
            return Err(BusinessError::InternalError { source: anyhow!(e) });
        }
    };

    Resp::ok(effect).to_json_result()
}
