---
title: 使用Rust、Actix-web和MongoDB构建简单博客网站-04
date: 2021-03-28
tags: [rust,actix-web,mongodb]
categories: rust
---

## 前言

距离上次更新有一年多了，很多依赖库的版本需要更新一下

本文完整源码见GITHUB Repo: https://github.com/nintha/demo-myblog

## 升级MongoDB驱动到官方版本

这个项目之前使用的MongoDB的Rust版本驱动是prototype版本，前段时间官方提供了正式版本（[github地址](<https://github.com/mongodb/mongo-rust-driver>)），我们可以升级一下。

修改下`Cargo.toml`，把`mongodb`的版本从`0.4.0`改成`0.9.0`

```toml
# mongodb = "0.4.0"
mongodb = "0.9.0"
```

重新编译下代码，发现大部分报错是API发生了变化，比如`Client`, `Collection`导入的路径，以及创建client实例的函数也需要修改.

```rust
// 官方版本驱动直接使用uri就可以构建新链接了
fn create_mongo_client() -> Client {
    Client::with_uri_str("mongodb://localhost:27017")
        .expect("Failed to initialize standalone client.")
}
```

以及指定database的API从`Client::db`变成`Client::database`。

```rust
// handles.rs
pub fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();
    let d: Document = struct_to_document(&article).unwrap();

    let result = collection(Article::TABLE_NAME).insert_one(d, None);
    match result {
        Ok(rs) => {
            /// InsertOneResult.inserted_id的类型发生了变化
            //let new_id: String = rs.inserted_id
            //    .and_then(|x| x.as_object_id().map(ObjectId::to_hex))
            //    .ok_or_else(|| {
            //        error!("save_article error, can not get inserted id");
            //        BusinessError::InternalError
            //    })?;
            let new_id: String = rs.inserted_id
                .as_object_id()
                .map(ObjectId::to_hex)
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
```

虽然版本号跳的比较多，但API的后向兼容性还是做得比较好的，我们不需要做过多的改动就可以让代码通过编译。



