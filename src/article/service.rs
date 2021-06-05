use crate::article::Article;
use crate::common::service::MongodbCrudService;
use crate::middleware::mongodb::DB_NAME;
use autowired::Autowired;
use mongodb::Collection;

#[derive(Default, autowired::Component)]
pub struct ArticleService {
    pub mongodb: Autowired<mongodb::Client>,
}

impl MongodbCrudService<Article> for ArticleService {

    fn table(&self) -> Collection {
        self.mongodb
            .database(DB_NAME)
            .collection(Article::TABLE_NAME)
    }

}
