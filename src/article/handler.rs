use actix_web::{web, HttpRequest};
use autowired::Autowired;
use bson::oid::ObjectId;
use bson::Document;

use crate::article::service::ArticleService;
use crate::article::ArticleQuery;
use crate::common::*;

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
        BusinessError::ValidationError("id".to_owned())
    })?;

    let effect = ARTICLE_SERVICE
        .update_by_oid(oid, &article.into_inner())
        .await?;
    log::info!("update article, id={}, effect={}", id, effect);

    Resp::ok(effect).to_json_result()
}

pub async fn remove_article(req: HttpRequest) -> RespResult {
    let id = req.match_info().get("id").unwrap_or("");
    if id.is_empty() {
        return Err(BusinessError::ValidationError("id".to_owned()));
    }

    let oid = ObjectId::with_string(id).map_err(|e| {
        log::error!("remove_article, can't parse id to ObjectId, {:?}", e);
        BusinessError::ValidationError("id".to_owned())
    })?;

    let deleted = ARTICLE_SERVICE.remove_by_oid(oid).await?;
    log::info!("delete article, id={}, effect={}", id, deleted);

    Resp::ok(deleted).to_json_result()
}
