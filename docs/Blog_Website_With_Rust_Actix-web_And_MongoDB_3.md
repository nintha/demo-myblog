---
title: 使用Rust、Actix-web和MongoDB构建简单博客网站-03
date: 2019-12-30
tags: [rust,actix-web,mongodb]
categories: rust
---

## 前言

上次留了一些问题，现在我们来解决一下。

- 升级MongoDB驱动到官方版本
- 升级Actix-web到2.0

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



## 升级Actix-web到2.0

在actix-web更换维护者后，unsafe事件终于尘埃落定，我们可以把手上actix-web项目都升级到2.0了。

修改下`Cargo.toml`

```toml
[dependencies]
#actix-web = "1.0"
actix-web = "2"
actix-rt = "1"
```

actix-web 2.0 已经支持async语法了，我们把相关函数都微调下。

main.rs

```rust
#[actix_rt::main]
async fn main() -> std::io::Result<()>{
    init_logger();

    let binding_address = "0.0.0.0:8000";
    HttpServer::new(|| App::new()
        .app_data(
              web::Json::<Article>::configure(|cfg| {
                  cfg.error_handler(|err, req| {
                      log::error!("json extractor error, path={}, {}", req.uri(), err);
                      BusinessError::ArgumentError.into()
                  })
              })
        )
        .service(
            web::scope("/articles")
                .route("", web::get().to(article::list_article))
                .route("", web::post().to(article::save_article))
                .route("{id}", web::put().to(article::update_article))
                .route("{id}", web::delete().to(article::remove_article))
        ))
        .bind(binding_address)
        .expect(&format!("Can not bind to {}", binding_address) )
        .run()
        .await
}
```

article/handle.rs
```rust
pub async fn save_article(article: web::Json<Article>) -> SimpleResp {...}

pub async fn list_article(query: web::Json<ArticleQuery>) -> SimpleResp {...}

pub async fn update_article(req: HttpRequest, article: web::Json<Article>) -> SimpleResp {...}

pub async fn remove_article(req: HttpRequest) -> SimpleResp {...}
```

好了，编译通过，升级完成。

actix-web的向下兼容还是很不错的，可能是这个项目比较简单，仅做少量微调就可以跑通了。



## 后记

现在项目中的错误处理有很多重复的地方，写起来也比较繁琐，定位异常也不太方便，计划后面对错误处理进行优化。

下一篇应该快了 （咕咕）。





