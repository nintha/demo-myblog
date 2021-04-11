use crate::article::Article;
use crate::common::{BusinessError, CursorIntoVec, Resp, RespResult};
use crate::middleware::mongodb::DB_NAME;
use autowired::Autowired;
use bson::Document;
use mongodb::Collection;

#[derive(Default, autowired::Component)]
pub struct ArticleService {
    pub mongodb: Autowired<mongodb::Client>,
}

impl ArticleService {

    pub fn table(&self) -> Collection {
        self.mongodb
            .database(DB_NAME)
            .collection(Article::TABLE_NAME)
    }

    pub async fn list_article(&self, filter: Document) -> RespResult {
        match self.table().find(Some(filter), None).await {
            Ok(cursor) => {
                let list = cursor.into_vec::<Article>().await;
                Resp::ok(list).to_json_result()
            }
            Err(e) => {
                log::error!("list_article error, {:?}", e);
                return Err(BusinessError::InternalError { source: anyhow!(e) });
            }
        }
    }
}
