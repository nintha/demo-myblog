---
title: 使用Rust、Actix-web和MongoDB构建简单博客网站-04
date: 2021-08-22
tags: [rust,actix-web,mongodb]
categories: rust
---

## 前言

距离上次更新有一年多了，本次文章计划是介绍使用`Autowired`库以及重构下项目结构。

本文完整源码见GITHUB Repo: https://github.com/nintha/demo-myblog

`autowired` GitHub地址：https://github.com/nintha/autowired-rs

## 依赖注入

web开发中有些是需要单例使用的组件，比如 mongodb 的 client。之前我们是直接用 `once_cell` 进行了一次简单的包装，但是如果这样的组件数量多了的话，维护起来也是一件头疼的事情。所以，我们将使用 `autowired` 库进行一些封装处理。

mongodb 相关的依赖先升级到新版本：

```toml
bson = "1.2"
mongodb = "1.2"
```

加入`autowired` 依赖：

```toml
autowired = "0.1.8"
inventory = "0.1"
```

这里引入的`inventory` 库是一个用于在main函数前执行一些逻辑的工具，`autowired` 库的实现依赖它。

```rust
// middleware/mongodb.rs

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
```

这里我们对一个返回值是`Option<Client>`的函数标注了属性`#[bean(option)]`，这样返回的对象会被 autowired 自动管理。

```rust
// main.rs
#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    autowired::setup_submitted_beans();
	
	// 其他逻辑代码
}
```

我们需要在程序开头的时候执行 `autowired::setup_submitted_beans()` ，用于初始化所有的组件。

```rust
// article/service.rs

#[derive(Default, autowired::Component)]
pub struct ArticleService {
    mongodb: Autowired<mongodb::Client>,
}

impl MongodbCrudService<Article> for ArticleService {

    fn table(&self) -> Collection {
        self.mongodb
            .database(DB_NAME)
            .collection(Article::TABLE_NAME)
    }

}
```

使用之前注册的单例组件也是比较简单的。这个例子是定义一个获取Collection实例的方法，由于`Autowired<Client>` 可以调用 `Deref::deref`获取到`&Client`，后续的使用就很自然了。可以看到，使用`autowired` 有一个小小的的限制，由于内部会把实例用`Arc<T>`进行一次封装，导致组件功能都需要支持通过`&self` 进行调用。

## CRUD抽象层

为了方便后续的开发和功能的扩展，我们把MongoDB的基本CRUD操作封装在一个Trait里面

```rust
// common/service.rs

use crate::common::{struct_into_document, CursorIntoVec};
use async_trait::async_trait;
use bson::oid::ObjectId;
use bson::Document;
use mongodb::Collection;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// type `T` is the record data type
///
/// Eg: it's the type `Article` for the `article` collection
#[async_trait(?Send)]
pub trait MongodbCrudService<T>
where
    T: 'static + DeserializeOwned + Serialize,
{
    fn table(&self) -> Collection;

    async fn list_with_filter(&self, filter: Document) -> mongodb::error::Result<Vec<T>> {
        let cursor = self.table().find(Some(filter), None).await?;
        Ok(cursor.into_vec::<T>().await)
    }

    /// return inserted id
    async fn save(&self, record: &T) -> anyhow::Result<String> {
        let d: Document = struct_into_document(record).ok_or_else(|| {
            anyhow!("[MongodbCrudService::save] Failed to convert struct into document")
        })?;
        let rs = self.table().insert_one(d, None).await?;
        let inserted_id: String = rs
            .inserted_id
            .as_object_id()
            .map(ObjectId::to_hex)
            .ok_or_else(|| anyhow!("[MongodbCrudService::save] Failed to get inserted id"))?;
        Ok(inserted_id)
    }

    /// return modified count
    async fn update_by_oid(&self, oid: ObjectId, record: &T) -> anyhow::Result<i64> {
        let filter = doc! {"_id": oid};

        let d: Document = struct_into_document(record).ok_or_else(|| {
            anyhow!("[MongodbCrudService::update_by_oid] Failed to convert struct into document")
        })?;
        let update = doc! {"$set": d};
        let result = self.table().update_one(filter, update, None).await?;
        Ok(result.modified_count)
    }

    /// return deleted count
    async fn remove_by_oid(&self, oid: ObjectId) -> anyhow::Result<i64> {
        let filter = doc! {"_id": oid};

        let result = self.table().delete_one(filter, None).await?;
        Ok(result.deleted_count)
    }
}
```

`MongodbCrudService` 定义了五个方法，除了` fn table(&self) -> Collection;` 没有默认的实现，剩下的CRUD4个方法都有默认的实现。使用的时候只需要声明具体的集合就可以了。

```RUST
// article/handler.rs

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

```

具体使用的时候，我们只需要关注业务逻辑，对输入输出参数进行一些逻辑校验和格式的转换，具体和MongoDB的交互就被屏蔽了，后续操作其他模型数据的时候也不需要写重复的代码。

