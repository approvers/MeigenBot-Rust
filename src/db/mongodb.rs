use crate::db::{MeigenDatabase, MeigenEntry, RegisteredMeigen};
use async_trait::async_trait;
use log::info;
use mongodb::bson::{doc, Bson, Document};
use mongodb::event::cmap::{CmapEventHandler, ConnectionClosedEvent, ConnectionCreatedEvent};
use mongodb::options::ClientOptions;
use mongodb::{Client, Collection};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::stream::StreamExt;

crate::make_error_enum! {
    MongoDBError;
    URLParseError url_parse_fail(e) => "MongoDBへのURLのパースに失敗しました: {}",
    OptionValidateFailError option_validate_fail(e) => "MongoDBへのOptionが不正です: {}",
    GettingMeigenError get_fail(e) => "MongoDBの名言の取得に失敗しました: {}",
    UpdatingMeigenError set_fail(e) => "MongoDBの名言の設定に失敗しました: {}",
    DeletingMeigenError delete_fail(e) => "MongoDBの名言の削除に失敗しました: {}",
    InvalidEntryError invalid_entry(e) => "MongoDBの中に無効なエントリがあります: {}",
    SerializeFailed serialize(e) => "Serializeに失敗しました: {}",
    DeserializeFailed deserialize(e) => "Deserializeに失敗しました: {}",
    NotFoundMeigen nf(id) => "ID{}を持つ名言はありません",
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
            id: data.id as i64, //safe: i64 range is in range of u32
            author: data.author,
            content: data.content,
        }
    }
}

impl Into<RegisteredMeigen> for MongoMeigen {
    fn into(self) -> RegisteredMeigen {
        RegisteredMeigen {
            id: self.id as u32, //safe: i64 range is in range of u32
            author: self.author,
            content: self.content,
        }
    }
}

struct MongoDBConnectionLogger {
    count: Mutex<u32>,
}

impl MongoDBConnectionLogger {
    fn new() -> Self {
        Self {
            count: Mutex::new(0),
        }
    }
}

impl CmapEventHandler for MongoDBConnectionLogger {
    fn handle_connection_created_event(&self, _: ConnectionCreatedEvent) {
        let mut count = self.count.lock().unwrap();
        *count += 1;

        info!(
            "New MongoDB connection created. current connection count: {}",
            count
        );
    }

    fn handle_connection_closed_event(&self, _: ConnectionClosedEvent) {
        let mut count = self.count.lock().unwrap();
        *count -= 1;

        info!(
            "MongoDB connection closed. current connection count: {}",
            count
        );
    }
}

#[derive(Debug, Clone)]
pub struct MongoDB {
    inner: Collection,
}

impl MongoDB {
    pub async fn new(url: &str) -> Result<MongoDB, MongoDBError> {
        let mut client_options = ClientOptions::parse(url)
            .await
            .map_err(MongoDBError::url_parse_fail)?;

        client_options.app_name = Some("Meigen Rust".into());
        client_options.cmap_event_handler = Some(Arc::new(MongoDBConnectionLogger::new()));
        client_options.min_pool_size = Some(0);
        client_options.max_pool_size = Some(1);
        client_options.max_idle_time = Some(Duration::from_secs(15));

        let database = Client::with_options(client_options)
            .map_err(MongoDBError::option_validate_fail)?
            .database("meigen");

        let entries = database.collection("entries");

        let result = MongoDB { inner: entries };

        Ok(result)
    }

    async fn search_by_doc(
        &self,
        doc: impl Into<Option<Document>>,
    ) -> Result<Vec<RegisteredMeigen>, MongoDBError> {
        let mut db_res = self
            .inner
            .find(doc, None)
            .await
            .map_err(MongoDBError::get_fail)?;

        let mut result = vec![];

        while let Some(entry) = db_res.next().await {
            let entry = entry.map_err(MongoDBError::get_fail)?;
            let deserialized =
                bson::from_bson(Bson::Document(entry)).map_err(MongoDBError::deserialize)?;
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

        let bson = bson::to_bson(&register_entry).map_err(MongoDBError::serialize)?;
        let doc = bson
            .as_document()
            .ok_or_else(|| MongoDBError::serialize("bson::to_bson returned not document"))?
            .clone();

        self.inner
            .insert_one(doc, None)
            .await
            .map_err(MongoDBError::set_fail)?;

        Ok(register_entry.into())
    }

    // 名言を削除する。
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error> {
        let result = self
            .inner
            .delete_one(
                doc! {
                    "id": Bson::Int64(id as i64)
                },
                None,
            )
            .await
            .map_err(MongoDBError::delete_fail)?;

        if result.deleted_count == 0 {
            return Err(MongoDBError::nf(id));
        }

        Ok(())
    }

    // 作者名から名言検索
    async fn search_by_author(&self, author: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "author": author }).await
    }

    // 名言本体から名言検索
    async fn search_by_content(&self, content: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "content": doc! { "$regex": format!(".*{}.*", content) } })
            .await
    }

    // idから名言取得
    async fn get_by_id(&self, id: u32) -> Result<RegisteredMeigen, Self::Error> {
        self.inner
            .find_one(doc! { "id": id }, None)
            .await
            .map_err(MongoDBError::get_fail)?
            .ok_or_else(|| MongoDBError::nf(id))
            .map(|x| bson::from_bson(Bson::Document(x)))?
            .map_err(MongoDBError::deserialize)
    }

    // idから名言取得(複数指定) 一致するIDの名言がなかった場合はスキップする
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.search_by_doc(doc! { "id": { "$in": ids } }).await
    }

    // 現在登録されている名言のなかで一番IDが大きいもの(=現在の(最大)名言ID)を返す
    async fn current_meigen_id(&self) -> Result<u32, Self::Error> {
        self.inner
            .aggregate(
                vec![doc! {
                    "$group": doc! {
                        "_id": "",
                        "current_id": doc! { "$max": "$id" }
                    }
                }],
                None,
            )
            .await
            .map_err(MongoDBError::get_fail)?
            .next()
            .await
            .ok_or_else(|| MongoDBError::get_fail("returned none"))?
            .map_err(MongoDBError::get_fail)?
            .get("current_id")
            .ok_or_else(|| MongoDBError::get_fail("mongodb didn't returned current_id"))?
            .as_i64()
            .ok_or_else(|| MongoDBError::get_fail("current_id wasn't Int64"))
            .map(|x| x as u32)
    }

    // len
    async fn len(&self) -> Result<u64, Self::Error> {
        self.inner
            .aggregate(vec![doc! { "$count": "id" }], None)
            .await
            .map_err(MongoDBError::get_fail)?
            .next()
            .await
            .ok_or_else(|| MongoDBError::get_fail("MongoDB returned none"))?
            .map_err(MongoDBError::get_fail)?
            .get("id")
            .ok_or_else(|| MongoDBError::get_fail("MongoDB response doesn't contain \"id\" field"))?
            .as_i64()
            .ok_or_else(|| MongoDBError::get_fail("MongoDB response's \"id\" field wasn't i64"))
            .map(|x| x as u64)
    }

    // 全名言取得
    async fn get_all_meigen(&self) -> Result<Vec<RegisteredMeigen>, MongoDBError> {
        self.search_by_doc(None).await
    }
}
