#![allow(dead_code)]

use crate::db::{MeigenDatabase, MeigenEntry, RegisteredMeigen};

use async_trait::async_trait;
use mongodb::bson::de::Error as BsonDeserializeError;
use mongodb::bson::ser::Error as BsonSerializeError;
use mongodb::bson::{self, doc, Bson, Document};
use mongodb::error::Error as MongoLibError;
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::stream::StreamExt;

trait ResultExt<T, E> {
    fn edit<F>(self, f: F) -> Self
    where
        F: FnOnce(&mut T);
}

impl<T, E> ResultExt<T, E> for Result<T, E> {
    fn edit<F>(mut self, f: F) -> Self
    where
        F: FnOnce(&mut T),
    {
        if let Ok(x) = self.as_mut() {
            f(x);
        }

        self
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MongoMeigen {
    id: i64,
    author: String,
    content: String,
}

impl From<RegisteredMeigen> for MongoMeigen {
    fn from(data: RegisteredMeigen) -> MongoMeigen {
        MongoMeigen {
            id: data.id as i64,
            author: data.author,
            content: data.content,
        }
    }
}

impl Into<RegisteredMeigen> for MongoMeigen {
    fn into(self) -> RegisteredMeigen {
        RegisteredMeigen {
            id: self.id as u32,
            author: self.author,
            content: self.content,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MongoDB {
    uri: String,
}

#[derive(Debug, Error)]
pub enum MongoDBError {
    #[error("MongoDBへのURIが無効です")]
    InvalidUri(#[source] MongoLibError),

    #[error("MongoDBからの情報の取得に失敗しました")]
    GetError(#[source] MongoLibError),

    #[error("MongoDBへの情報の保存に失敗しました")]
    SaveError(#[source] MongoLibError),

    #[error("MongoDBのエントリの削除に失敗しました")]
    DeleteError(#[source] MongoLibError),

    #[error("MongoDBからの応答のシリアライズに失敗しました")]
    SerializeError(#[source] BsonSerializeError),

    #[error("MongoDBからの応答のデシリアライズに失敗しました")]
    DeserializeError(#[source] BsonDeserializeError),

    #[error("削除要求された名言(id: {id})は見つかりませんでした")]
    DeleteTargetNotFound { id: u32 },

    #[error("多分かわえもんのミスです。ごめんなさい。")]
    Bug(&'static str),
}

impl MongoDB {
    pub async fn new(uri: &str) -> Result<MongoDB, MongoDBError> {
        let result = Self {
            uri: uri.to_string(),
        };

        // test run
        let _ = result.connect().await?;

        Ok(result)
    }

    async fn connect(&self) -> Result<Collection, MongoDBError> {
        let mut client_options = ClientOptions::parse(&self.uri)
            .await
            .map_err(MongoDBError::InvalidUri)?;

        client_options.app_name = Some("Meigen Rust".into());

        let collection = Client::with_options(client_options)
            .map_err(MongoDBError::InvalidUri)?
            .database("meigen")
            .collection("entries");

        Ok(collection)
    }

    async fn search_by_doc(
        &self,
        doc: impl Into<Option<Document>>,
    ) -> Result<Vec<RegisteredMeigen>, MongoDBError> {
        let mut db_res = self
            .connect()
            .await?
            .find(doc, None)
            .await
            .map_err(MongoDBError::GetError)?;

        let mut result = vec![];

        while let Some(entry) = db_res.next().await {
            let entry = entry.map_err(MongoDBError::GetError)?;
            let deserialized =
                bson::from_bson(Bson::Document(entry)).map_err(MongoDBError::DeserializeError)?;
            result.push(deserialized);
        }

        Ok(result)
    }
}

#[async_trait]
impl MeigenDatabase for MongoDB {
    type Error = MongoDBError;

    // 名言を保存する。
    async fn save_meigen(&mut self, entry: MeigenEntry) -> Result<RegisteredMeigen, Self::Error> {
        let current_id = self.current_meigen_id().await? as u32;

        let register_entry = MongoMeigen {
            id: (current_id + 1) as i64,
            author: entry.author,
            content: entry.content,
        };

        let doc = bson::to_document(&register_entry).map_err(MongoDBError::SerializeError)?;

        self.connect()
            .await?
            .insert_one(doc, None)
            .await
            .map_err(MongoDBError::SaveError)?;

        Ok(register_entry.into())
    }

    // 名言を削除する。
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error> {
        let result = self
            .connect()
            .await?
            .delete_one(doc! { "id": Bson::Int64(id as i64) }, None)
            .await
            .map_err(MongoDBError::DeleteError)?;

        if result.deleted_count == 0 {
            return Err(MongoDBError::DeleteTargetNotFound { id });
        }

        Ok(())
    }

    // 作者名から名言検索
    async fn search_by_author(&self, author: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "author": { "$regex": format!(".*{}.*", author) }})
            .await
            .edit(|x| x.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap()))
    }

    // 名言本体から名言検索
    async fn search_by_content(&self, content: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "content": { "$regex": format!(".*{}.*", content) }})
            .await
            .edit(|x| x.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap()))
    }

    // idから名言取得
    async fn get_by_id(&self, id: u32) -> Result<Option<RegisteredMeigen>, Self::Error> {
        self.connect()
            .await?
            .find_one(doc! { "id": id }, None)
            .await
            .map_err(MongoDBError::GetError)?
            .map(bson::from_document)
            .transpose()
            .map_err(MongoDBError::DeserializeError)
    }

    // idから名言取得(複数指定) 一致するIDの名言がなかった場合はスキップする
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "id": { "$in": ids } })
            .await
            .edit(|x| x.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap()))
    }

    // 現在登録されている名言のなかで一番IDが大きいもの(=現在の(最大)名言ID)を返す
    async fn current_meigen_id(&self) -> Result<u32, Self::Error> {
        self.connect()
            .await?
            .aggregate(
                vec![doc! {
                    "$group": {
                        "_id": "",
                        "current_id": {
                            "$max": "$id"
                        }
                    }
                }],
                None,
            )
            .await
            .map_err(MongoDBError::GetError)?
            .next()
            .await
            .ok_or_else(|| MongoDBError::Bug("MongoDBからのレスポンスが空でした"))?
            .map_err(MongoDBError::GetError)?
            .get("current_id")
            .ok_or_else(|| {
                MongoDBError::Bug(
                    "MongoDBからのレスポンスに current_id フィールドが含まれていませんでした",
                )
            })?
            .as_i64()
            .ok_or_else(|| MongoDBError::Bug("current_id フィールドがi64ではありませんでした"))
            .map(|x| x as u32)
    }

    // len
    async fn len(&self) -> Result<u64, Self::Error> {
        self.connect()
            .await?
            .aggregate(vec![doc! { "$count": "id" }], None)
            .await
            .map_err(MongoDBError::GetError)?
            .next()
            .await
            .ok_or_else(|| MongoDBError::Bug("MongoDBからのレスポンスが空でした"))?
            .map_err(MongoDBError::GetError)?
            .get("id")
            .ok_or_else(|| {
                MongoDBError::Bug("MongoDBからのレスポンスに id フィールドが含まれていませんでした")
            })?
            .as_i32()
            .ok_or_else(|| MongoDBError::Bug("id フィールドがi32ではありませんでした"))
            .map(|x| x as u64)
    }

    // 全名言取得
    async fn get_all_meigen(&self) -> Result<Vec<RegisteredMeigen>, MongoDBError> {
        self.search_by_doc(None)
            .await
            .edit(|x| x.sort_by(|a, b| a.id.partial_cmp(&b.id).unwrap()))
    }
}
