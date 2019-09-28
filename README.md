# demo-myblog

## 前言

本文介绍如何使用Actix-web和MongoDB构建简单博客网站。其中Actix-web 是一个高效的 HTTP Server 框架（[Web Framework Benchmarks](<https://www.techempower.com/benchmarks/#section=data-r18>) 上位居榜首），Mongodb是一个流行的数据库软件。

## 开始

我们使用`cargo`包管理工具来创建项目，当前的rust版本为v1.38

```shell
cargo new myblog
```

创建成功后 myblog目录结构如下所示

```shell
myblog/
├── .git/
├── .gitignore
├── Cargo.toml
└── src
    └── main.rs
```

### 日志打印

为了方便项目开发，日志输出必不可少，光靠`println!`可不行，这里我们引入日志扩展依赖，在Cargo.toml文件中添加:

```toml
log = "0.4.0"
env_logger = "0.6.0"
chrono = "0.4.9"
```

然后在`main.rs`里添加日志初始化相关代码

```rust
use log::info;

fn init_logger() {
    use chrono::Local;
    use std::io::Write;

    let env = env_logger::Env::default()
        .filter_or(env_logger::DEFAULT_FILTER_ENV, "info");
	// 设置日志打印格式
    env_logger::Builder::from_env(env)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} {} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.module_path().unwrap_or("<unnamed>"),
                &record.args()
            )
        })
        .init();
    info!("env_logger initialized.");
}

fn main() {
    init_logger();
    info!("hello world");
}
```

我们运行下，看看效果

```
2019-09-28 14:12:40 INFO [myblog] env_logger initialized.
2019-09-28 14:12:40 INFO [myblog] hello world
```

嗯，友好的日志信息。

### 创建HTTP服务

现在引入actix-web所需要的依赖，在Cargo.toml文件中添加依赖：

```
actix-web = "1.0"
```

根据Actix官网的示例代码，创建http server的代码如下所示：

```rust
use actix_web::{web, App, HttpRequest, HttpServer, Responder};

fn greet(req: HttpRequest) -> impl Responder {
    let name = req.match_info().get("name").unwrap_or("World");
    format!("Hello {}!", &name)
}

fn main() {
    init_logger();
    info!("hello world");

    let binding_address = "0.0.0.0:8000";
    let server = HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(greet))
            .route("/{name}", web::get().to(greet))
    })
        .bind(binding_address)
        .expect("Can not bind to port 8000");

    server.run().unwrap();
}
```

运行下程序，用浏览器访问`http://localhost:8000/`，不出意外的话可以看到应答`Hello World!`。

### 请求异常处理

为了代码更加健壮，我们需要对请求的异常处理进行自定义。

在src下面添加`common.rs`文件，并在`main.rs`中声明这个模块

```rust
mod common;
```

我们使用了`failure`库来辅助错误处理以及`serde`库对请求应答进行序列化，在依赖中加入它们

```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
failure = "0.1.5"
```

我们定义个统一的返回值结构体`Resp`，代码如下所示

```rust
#[derive(Deserialize, Serialize)]
pub struct Resp<T> where T: Serialize {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> Resp<T> {
    pub fn ok(data: T) -> Self {
        Resp { code: 0, message: "ok".to_owned(), data: Some(data) }
    }

    pub fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    pub fn err(error: i32, message: &str) -> Self {
        Resp { code: error, message: message.to_owned(), data: None }
    }
}
```

当请求正常处理的时候，用`ok()`进行返回

```rust
Resp::ok("success").to_json_result()
```

当出现业务错误的时候，如请求参数缺失，用`err()`进行返回

```rust
Resp::err(err_code, "error message").to_json_result()
```

如果需要其他HTTP Response Code，比如404，可以这样写

```rust
HttpResponse::NotFound().json(Resp::err(err_code, "error message") // code 404
```

同时我们需要自定义下业务异常

```rust
#[derive(Fail, Debug)]
pub enum BusinessError {
    #[fail(display = "Validation error on field: {}", field)]
    ValidationError { field: String },
    #[fail(display = "An internal error occurred. Please try again later.")]
    InternalError,
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            BusinessError::ValidationError { .. } => {
                let resp = Resp::err(10001, &self.to_string());
                HttpResponse::BadRequest().json(resp)
            }
            _ => {
                let resp = Resp::err(10000, &self.to_string());
                HttpResponse::InternalServerError().json(resp)
            }
        }
    }

    // 重写response的序列化结果
    fn render_response(&self) -> HttpResponse {
        self.error_response()
    }
}
```

这里用枚举定义了两种业务错误，`ValidationError`表示请求参数校验错误，`InternalError`作为普通内部错误。

枚举值上的注解属性`#[fail(display = "Validation error on field: {}", field)]`，是用了failure的功能，可以让错误信息更加友好，动态的错误信息可以更加直观的看到出错的具体参数信息。

`error::ResponseError`是Actix-web处理错误返回的trait，`fn error_response(&self) -> HttpResponse`方法是对错误进行处理，把我们自己定义的错误转换成`Actix`可以处理的错误；`fn render_response(&self) -> HttpResponse`是对错误信息进行序列化，成为前端接受到的内容。如果不重载`render_response`，返回到前端的只会是`#[fail(display = "Validation error on field: {}", field)]`中`display`的部分，这样就很不JSON了。

`common.rs` 完整内容

```rust
/// common.rs

use actix_web::{HttpResponse, error};
use serde::{Serialize, Deserialize};
use failure::Fail;

#[derive(Fail, Debug)]
pub enum BusinessError {
    #[fail(display = "Validation error on field: {}", field)]
    ValidationError { field: String },
    #[fail(display = "An internal error occurred. Please try again later.")]
    InternalError,
}

impl error::ResponseError for BusinessError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            BusinessError::ValidationError { .. } => {
                let resp = Resp::err(10001, &self.to_string());
                HttpResponse::BadRequest().json(resp)
            }
            _ => {
                let resp = Resp::err(10000, &self.to_string());
                HttpResponse::InternalServerError().json(resp)
            }
        }
    }
    // 重写response的序列化结果
    fn render_response(&self) -> HttpResponse {
        self.error_response()
    }
}

#[derive(Deserialize, Serialize)]
pub struct Resp<T> where T: Serialize {
    code: i32,
    message: String,
    data: Option<T>,
}

impl<T: Serialize> Resp<T> {
    pub fn ok(data: T) -> Self {
        Resp { code: 0, message: "ok".to_owned(), data: Some(data) }
    }

    pub fn to_json_result(&self) -> Result<HttpResponse, BusinessError> {
        Ok(HttpResponse::Ok().json(self))
    }
}

impl Resp<()> {
    pub fn err(error: i32, message: &str) -> Self {
        Resp { code: error, message: message.to_owned(), data: None }
    }
}
```



### 集成MongoDB

这里假设用户已经在本地已经有一个MongoDB服务器，可以通过`mongodb://localhost:27017`进行访问，并且未设置密码。

添加依赖

```toml
bson = "0.14.0"
mongodb = "0.4.0"
lazy_static = "1.4.0"
```

这里添加了`lazy_static`的依赖主要是希望可以把MongoDB Client作为一个全局变量进行复用，

```rust
use lazy_static::lazy_static;
use mongodb::{Client, ThreadedClient};

lazy_static! {
    pub static ref MONGO: Client = create_mongo_client();
}

fn create_mongo_client() -> Client {
    Client::connect("localhost", 27017)
        .expect("Failed to initialize standalone client.")
}
```

`mongodb::Client`类型其实是`Arc<ClientInner>`类型的别名，所以它可以在多个线程内安全地共享。

对于这个简单的项目我们只会用到一个database，所以把database的访问也可以封装一下:

```rust
use mongodb::db::ThreadedDatabase;
use mongodb::coll::Collection;

fn collection(coll_name: &str) -> Collection {
    MONGO.db("myblog").collection(coll_name)
}
```

这样我们只需要关注`集合（monogodb collection）`的逻辑就可以了，比如查询user集合的数据量，可以这么写

```rust
let rs = collection("user").count(None, None);
info!("count={}", rs.unwrap()); 
```



### CRUD

我们的目标是完成一个博客，那么最基础功能是提供增删改查4个API。博客最主要的内容就是文章，因此我们先创建`Article`结构体来描述文章这个实例。

在`src`下面创建`article`文件夹，并在`article`文件夹下面创建`mod.rs`和`handler.rs`文件，现在src的目录结构是这样的

```
src/
├── article/
│   ├── handler.rs
│   └── mod.rs
└── main.rs
```

`mod.rs` 文件是用来定义article模块中共用的部分，`handler.rs `文件用于存放请求处理相关的代码。

我们先看下mod.rs

```rust
mod handler;

pub use handler::*;
use bson::oid::ObjectId;

#[derive(Debug)]
pub struct Article {
    _id: Option<ObjectId>,
    title: String,
    author: String,
    content: String,
}
```

我们定义了`Article`结构体，它包含了4个字段，`_id`是由MongoDB自动生成的，但在文章创建前，它是不存在的，所以我们用`Option`包裹一下。为了方便，这个结构体不仅用于前端请求参数的接受，同时用于响应数据的返回，还用于同步数据库的模型。

由于我们希望对应的表名为`article`，那么为`Article`实现一个常量字符串;

```rust
impl Article {
    pub const TABLE_NAME: &'static str = "article";
}
```

现在可以尝试下编写新增逻辑了，先决定方法声明，如下所示

```rust
pub fn save_article(article: web::Json<Article>) -> Result<HttpResponse, BusinessError> 
```

这个返回类型看起来有点长，而且基本不会改变，那我们可以用类型别名去简化

```rust
type SimpleResp = Result<HttpResponse, BusinessError>;

pub fn save_article(article: web::Json<Article>) -> SimpleResp
```

 这下就简单多了。

` web::Json<Article>`是actix提供用来接受json body的对象，可以用`::into_inner()`方法直接获取反序列化好的结构体

```rust
pub fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();
    
}
```

我们先测试下是否真的可以拿到请求的参数，把代码稍微补充一下：

```rust
use super::Article;
use actix_web::{HttpResponse, web};
use log::*;
use crate::common::*;

type SimpleResp = Result<HttpResponse, BusinessError>;

pub fn save_article(article: web::Json<Article>) -> SimpleResp {
    let article: Article = article.into_inner();

    info!("save article, {:?}", article);
    Resp::ok(article.title).to_json_result()
}
```

还需要在main.rs里面把handler绑定到路由上(hello world已经不在需要，这里先移除了)

```rust
fn main() {
    init_logger();

    let binding_address = "0.0.0.0:8000";
    let server = HttpServer::new(|| {
        App::new().service(
            web::scope("/articles")
                .route("", web::post().to(article::save_article))
        )
    })
        .bind(binding_address)
        .expect("Can not bind to port 8000");

    server.run().unwrap();
}
```

我们把`save_article`方法绑定到`POST /articles`路由上，但是这样却没法通过编译

```shell
...
error[E0277]: the trait bound `for<'de> article::Article: common::_IMPL_SERIALIZE_FOR_Resp::_serde::Deserialize<'de>` is not satisfied
  --> src\main.rs:55:44
   |
55 |                     .route("", web::post().to(article::save_article))
   |                                            ^^ the trait `for<'de> common::_IMPL_SERIALIZE_FOR_Resp::_serde::Deserialize<'de>` is not implemented for `article::Article`
   |
   = note: required because of the requirements on the impl of `common::_IMPL_SERIALIZE_FOR_Resp::_serde::de::DeserializeOwned` for `article::Article`
   = note: required because of the requirements on the impl of `actix_web::extract::FromRequest` for `actix_web::types::json::Json<article::Article>`
   = note: required because of the requirements on the impl of `actix_web::extract::FromRequest` for `(actix_web::types::json::Json<article::Article>,)`

error: aborting due to previous error
...
```

友善的编译器告诉我们，`article::Article`结构体提没有实现反序列化相关方法；从json变成article的确需要反序列化，如果我们需要把article作为结果返回，同时还需要序列化，接下来就实现一下

```rust
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct Article {
	...
}
```

我们只需要声明`Article`实现了` serde::Serialize`和` serde:: Deserialize`特性，然后serde就会帮我们自动完成背后的工作。现在项目可以正常启动了，尝试发送一个post请求

```shell
curl --request POST \
  --url http://172.28.224.1:8000/articles \
  --header 'Content-Type: application/json' \
  --data '{"title": "简易博客指南","author": "栗子球","content": "本文介绍如何使用Actix-web和MongoDB构建简单博客网站..."}'
```

可以看到一条日志，这个请求参数已经被我们成功获取并打印了。

```shell
2019-09-28 20:52:19 INFO [myblog::article::handler] save article, Article { _id: None, title: "简易博客指南", author: "栗子球", content: "本文介绍如何使用Actix-web和MongoDB构建简单博客网站..." }
```

然后就是需要写入数据库了，当前rust上mongodb实现，在进行所有操作时，需要把结构体转换成`Doucument`类型。同时我们需要对`_id`字段进行移除，不然mongodb无法生成对应 ID了。

```rust
// Article -> Bson -> Document
let mut d = bson::to_bson(&article)
        .map(|x| x.as_document().unwrap().to_owned())
        .unwrap();
d.remove("_id");
let result = collection(Article::TABLE_NAME).insert_one(d, None);
```

写入数据库后，返回值会告诉我们这条记录的ID，同时需要对失败情况进行处理

```rust
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
```

我们再次运行程序，发送请求，成功的话响应json数据如下所示

```json
{
    "code": 0,
    "message": "ok",
    "data": "5d8f5ff300368817005c82a2"
}
```

接下来处理查询接口，把我们刚刚存储的数据查询出来，`Collection::find`方法返回的值是一个游标（`mongodb::cursor::Cursor`），我们可以把它转换成Vec，在common.rs里面添加如下代码

```rust
use bson::Document;

pub trait CursorToVec {
    fn to_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T>;
}

impl CursorToVec for mongodb::cursor::Cursor {
    fn to_vec<'a, T: Serialize + Deserialize<'a>>(&mut self) -> Vec<T> {
        self.map(|item| {
            let doc: Document = item.unwrap();
            let bson = bson::Bson::Document(doc);
            return bson::from_bson(bson).unwrap();
        }).collect()
    }
}

```

由于rust的孤儿原则，我们定义了一个新的trait，来为游标类型实现扩展方法。

查询处理如下所示， 我们仅仅是不加过滤参数地查询一下，把游标转换成动态数组，在对错误进行一下处理。

```rust
pub fn list_article() -> SimpleResp {
    let coll = collection("article");

    let cursor = coll.find(Some(doc! {}), None);
    let result = cursor.map(|mut x| x.to_vec::<Article>());
    match result {
        Ok(list) => Resp::ok(list).to_json_result(),
        Err(e) => {
            error!("list_article error, {}", e);
            return Err(BusinessError::InternalError);
        }
    }
}
```

在`main.rs`中绑定新路由，这次绑定到GET 上

```rust
let server = HttpServer::new(|| {
    App::new().service(
        web::scope("/articles")
        .route("", web::post().to(article::save_article))
        .route("", web::get().to(article::list_article))
    )
})
```

用GET请求`http://127.0.0.1:8000/articles`，获得响应

```json
{
    "code": 0,
    "message": "ok",
    "data": [
        {
            "_id": {
                "$oid": "5d8f5ff300368817005c82a2"
            },
            "title": "简易博客指南",
            "author": "栗子球",
            "content": "本文介绍如何使用Actix-web和MongoDB构建简单博客网站..."
        }
    ]
}
```

可以看到数据已经被完整的读出，美中不足的是`_id`字段显示不太符合我们的直觉；我们希望它直接显示那一段hash值，而不是一个嵌套字段。通过查询serde的文档可以得知，我们可以通过注释字段来处理某个字段的序列化方式。

```rust
use serde::Serializer;

#[derive(Deserialize, Serialize, Debug)]
pub struct Article {
    #[serde(serialize_with = "serialize_object_id")]
    _id: Option<ObjectId>,
    title: String,
    author: String,
    content: String,
}

pub fn serialize_object_id<S>(oid: &Option<ObjectId>, s: S) -> Result<S::Ok, S::Error> where S: Serializer {
    match oid.as_ref().map(|x| x.to_hex()) {
        Some(v) => s.serialize_str(&v),
        None => s.serialize_none()
    }
}
```

现在再来看看效果

```json
{
    "code": 0,
    "message": "ok",
    "data": [
        {
            "_id": "5d8f5ff300368817005c82a2",
            "title": "简易博客指南",
            "author": "栗子球",
            "content": "本文介绍如何使用Actix-web和MongoDB构建简单博客网站..."
        }
    ]
}
```

这下看起来舒服多了。



为了方便把变量转换成Document,我们把这部分逻辑提取出来。这里做了一个额外处理，就是把所有空值的key都删除，方便后续业务处理。

```rust
// article/handler.rs

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
```

字符串转换ObjectId有个错误，为了方便使用`?`语法，我们添加了`bson::oid::Error`到`BusinessError`的转换.

```rust
// common.rs

impl std::convert::From<bson::oid::Error> for BusinessError {
    fn from(_: bson::oid::Error) -> Self {
        BusinessError::InternalError
    }
}
```



然后就照葫芦画瓢把修改和删除写一下

```rust
// article/handle.rs

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
```

我们把修改逻辑绑定到 `PUT /articles/{id}`，删除逻辑绑定到 `DELETE /articles/{id}`，获取路径变量可以通过`HttpRequest.match_info(&self).get("id")`来获取

```rust
// main.rs

let server = HttpServer::new(|| {
    App::new().service(
        web::scope("/articles")
        .route("", web::get().to(article::list_article))
        .route("", web::post().to(article::save_article))
        .route("{id}", web::put().to(article::update_article))
        .route("{id}", web::delete().to(article::remove_article))
    )
})
```



## 后记

现在基本功能已经完成了，但还留有一些小小的问题

- 带条件参数的查询
- 请求时JSON格式异常或缺字段时，返回的信息不是JSON格式的
- 缺少前端页面
- ……








