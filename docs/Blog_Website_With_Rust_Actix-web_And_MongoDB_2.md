---
title: 使用Rust、Actix-web和MongoDB构建简单博客网站-02
date: 2019-11-17
tags: [rust,actix-web,mongodb]
categories: rust
---

## 前言

上次留了一些问题，现在我们来解决一下。

- 对请求时的反序列化异常进行处理
- 带条件参数的查询接口

本文完整源码见GITHUB Repo: https://github.com/nintha/demo-myblog

## 对请求时的反序列化异常进行处理

问题复现：

我们构造一个这样的请求，body里面的JSON字符串比正常少了一个`content`字段

```
POST /articles
body {
	"title": "简易博客指南",
	"author": "栗子球"
}
```

响应内容:
```
Json deserialize error: trailing comma at line 5 column 1
```

这是普通文本，我们需要把这个错误信息捕获并转换为统一的错误格式

在 `main`函数里面修改

```rust
// main.rs
fn main() {
    init_logger();

    let binding_address = "0.0.0.0:8000";
    let server = HttpServer::new(|| App::new()
        // change json extractor configuration
        .data(
              web::Json::<Article>::configure(|cfg| {
                  cfg.error_handler(|err, _req| {
                      // <- create custom error response
                      log::error!("json extractor error, path={}, {}", _req.uri(), err);
                      let resp = Resp::err(10002, "argument error");
                      error::InternalError::from_response(
                          err,
                          HttpResponse::BadRequest().json(resp),
                      ).into()
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
        .expect("Can not bind to port 8000");

    server.run().unwrap();
}
```

这里调用了`.data(..)`方法，对JSON反序列化的异常进行自定义处理，对于原始错误信息使用日志进行打印，对于前端返回内容，则是用我们自定义错误信息

```json
{
    "code": 10002,
    "message": "argument error",
    "data": null
}
```



## 带条件参数的查询接口

首先我们声明一个新的结构体，用来承载查询条件参数

```rust
// article/mod.rs

#[derive(Deserialize, Serialize, Debug)]
pub struct ArticleQuery {
    #[serde(deserialize_with = "deserialize_object_id", default)]
    _id: Option<ObjectId>,
    #[serde(default)]
    keyword: String,
}
```

`_id`用于精确匹配，`keyword`用于模糊匹配`title/author/content`字段内容。

由于这些参数是可选的，所以`_id`用`Option`包一层，`keyword` 是字符串，可以直接用默认值进行处理。`Option`的默认值是`None`，`String` 的默认值是空字符串。

`ObjectId`默认的反序列化肯定是不好用的，我们希望用户传递一个字符串就可以了，所以需要对`_id`字段指定自定义反序列化函数。反序列化函数如下所示：

```rust
// article/mod.rs

pub fn deserialize_object_id<'de, D>(deserializer: D) -> Result<Option<ObjectId>, D::Error>
    where D: Deserializer<'de> {
        
    struct JsonOptionObjectIdVisitor;

    impl<'de> de::Visitor<'de> for JsonOptionObjectIdVisitor {
        type Value = Option<ObjectId>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("an object id hash value")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: de::Error {
            if v.is_empty() {
                return Ok(None);
            }
            Ok(ObjectId::with_string(v).ok())
        }
    }

    deserializer.deserialize_any(JsonOptionObjectIdVisitor)
}
```

然后就可以使用这个定义好的结构体了，把之前的查询接口函数

```rust
pub fn list_article() -> SimpleResp {
    ..
}
```

改成

```rust
pub fn list_article(query: web::Json<ArticleQuery>) -> SimpleResp {
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
            return Err(BusinessError::InternalError);
        }
    }
}
```

这里我们先判断下每个参数是否有传递上来，对于没有的参数进行忽略。模糊匹配这里使用了大小写不敏感正则匹配，为了覆盖多个字段，则是用了`mongodb` 提供的`$or`逻辑操作符。

测试下效果，这里使用了带 body的GET请求，首先是空参数

```
GET /articles
body {}
```

响应内容:

```json
{
    "code": 0,
    "message": "ok",
    "data": [
        {
            "_id": "5d8f70a10009c1f200be8cae",
            "title": "七天学会Rust",
            "author": "noone",
            "content": "如果七天学不会，那就再学七天"
        },
        {
            "_id": "5d8f76ed00e36d9600b7604d",
            "title": "简易博客指南",
            "author": "栗子球",
            "content": "本文介绍如何使用Actix-web和MongoDB构建简单博客网站..."
        },
        {
            "_id": "5dca98f100fe6fda00f51150",
            "title": "rust从入门到放弃",
            "author": "佚名",
            "content": "Hello World"
        }
    ]
}
```

带上`_id`参数的请求

```
GET /articles
body {
    "_id": "5d8f70a10009c1f200be8cae"
}
```

响应内容:

```json
{
    "code": 0,
    "message": "ok",
    "data": [
        {
            "_id": "5d8f70a10009c1f200be8cae",
            "title": "七天学会Rust",
            "author": "noone",
            "content": "如果七天学不会，那就再学七天"
        }
    ]
}
```

带上`keyword`参数的请求

```
GET /articles
body {
    "keyword": "rust"
}
```

响应内容:

```json
{
    "code": 0,
    "message": "ok",
    "data": [
        {
            "_id": "5d8f70a10009c1f200be8cae",
            "title": "七天学会Rust",
            "author": "noone",
            "content": "如果七天学不会，那就再学七天"
        },
        {
            "_id": "5dca98f100fe6fda00f51150",
            "title": "rust从入门到放弃",
            "author": "佚名",
            "content": "Hello World"
        }
    ]
}
```



## 后记

这次我们解决了上次遗留下来的一部分问题。很多时候解决一个问题会引发另一个问题，比如为了模糊查询，我们这里使用了正则匹配+`$or`来实现的，可能在数据量比较大的时候性能表现并不理想，到时候要考虑是否替换成全文检索进行实现。

下一篇应该快了 （咕咕）。





