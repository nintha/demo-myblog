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
