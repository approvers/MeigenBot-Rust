use crate::db::MeigenDatabase;
use crate::db::MeigenEntry;
use crate::db::RegisteredMeigen;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::bson::Bson;
use mongodb::options::ClientOptions;
use mongodb::Client;
use mongodb::Collection;
use serde::Deserialize;
use serde::Serialize;
use std::convert::TryInto;
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

pub struct MongoDB {
    inner: Collection,
}

impl MongoDB {
    pub async fn new(url: &str) -> Result<MongoDB, MongoDBError> {
        let mut client_options = ClientOptions::parse(url)
            .await
            .map_err(MongoDBError::url_parse_fail)?;

        client_options.app_name = Some("Meigen Rust".into());

        let database = Client::with_options(client_options)
            .map_err(MongoDBError::option_validate_fail)?
            .database("meigen");

        let entries = database.collection("entries");

        let result = MongoDB { inner: entries };

        Ok(result)
    }

    pub async fn get_all_meigen(&self) -> Result<Vec<RegisteredMeigen>, MongoDBError> {
        let mut meigens = vec![];
        let mut cursor = self
            .inner
            .find(None, None)
            .await
            .map_err(MongoDBError::get_fail)?;

        while let Some(doc) = cursor.next().await {
            let doc = doc.map_err(MongoDBError::get_fail)?;
            let bson = bson::Bson::Document(doc);
            let meigen = bson::from_bson::<MongoMeigen>(bson).map_err(MongoDBError::deserialize)?;

            meigens.push(meigen.into());
        }

        Ok(meigens)
    }
}

#[async_trait]
impl MeigenDatabase for MongoDB {
    type Error = MongoDBError;

    // 名言を保存する。
    async fn save_meigen(&mut self, entry: MeigenEntry) -> Result<RegisteredMeigen, Self::Error> {
        let current_id = self
            .inner
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
            .ok_or_else(|| MongoDBError::get_fail("not returned current_id"))?
            .as_i64()
            .ok_or_else(|| MongoDBError::get_fail("current_id wasn't Int64"))?
            as u32; //safe: i64 range is in range of u32

        let current_id: u32 = current_id
            .try_into()
            .map_err(|_| MongoDBError::get_fail("current_id wasn't in u32 range"))?;

        let register_entry = MongoMeigen {
            id: (current_id + 1) as i64, //safe: i64 range is in range of u32
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
        let result = self.inner
            .delete_one(
                doc! {
                    "id": Bson::Int64(id as i64) //safe: i64 range is in range of u32
                },
                None,
            )
            .await
            .map_err(MongoDBError::delete_fail)?;

        if result.deleted_count == 0 {
            return Err(MongoDBError::nf(id))
        }

        Ok(())
    }

    // 名言スライスを返す。
    async fn meigens(&self) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        self.get_all_meigen().await
    }
}
