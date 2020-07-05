use crate::db::MeigenDatabase;
use crate::db::MeigenEntry;
use crate::db::RegisteredMeigen;
use async_trait::async_trait;
use mongodb::bson::doc;
use mongodb::bson::Bson;

use mongodb::options::ClientOptions;
use mongodb::Client;
use mongodb::Collection;
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
}

pub struct MongoDB {
    inner: Collection,
    current_id: u32,
    list: Vec<RegisteredMeigen>,
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

        let mut result = MongoDB {
            inner: entries,
            current_id: 0,
            list: vec![],
        };

        result.sync().await?;

        Ok(result)
    }

    // db -> Self をする。 Self -> dbはしないので注意
    pub async fn sync(&mut self) -> Result<(), MongoDBError> {
        let mut meigens: Vec<RegisteredMeigen> = vec![];
        let mut cursor = self
            .inner
            .find(None, None)
            .await
            .map_err(MongoDBError::get_fail)?;

        while let Some(doc) = cursor.next().await {
            let doc = doc.map_err(MongoDBError::get_fail)?;
            let bson = bson::Bson::Document(doc);
            let meigen = bson::from_bson(bson).map_err(MongoDBError::deserialize)?;

            meigens.push(meigen);
        }

        self.current_id = meigens
            .iter()
            .fold(0, |a, b| if a < b.id { b.id } else { a });
        self.list = meigens;

        Ok(())
    }
}

#[async_trait]
impl MeigenDatabase for MongoDB {
    type Error = MongoDBError;

    // 名言を保存する。
    async fn save_meigen(&mut self, entry: MeigenEntry) -> Result<&RegisteredMeigen, Self::Error> {
        self.sync().await?;
        self.current_id += 1;

        let register_entry = RegisteredMeigen {
            id: self.current_id,
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

        self.list.push(register_entry);

        Ok(self.list.iter().last().unwrap())
    }

    // 名言を削除する。
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error> {
        self.sync().await?;

        let list_pos = self
            .list
            .iter()
            .position(|x| x.id == id)
            .ok_or_else(|| MongoDBError::delete_fail("such id not found"))?;

        self.list.remove(list_pos);

        let id_int64: i64 = id.into();
        self.inner
            .delete_one(doc! {"id": Bson::Int64(id_int64)}, None)
            .await
            .map_err(MongoDBError::delete_fail)?;

        Ok(())
    }

    // 名言スライスを返す。
    async fn meigens(&self) -> &[RegisteredMeigen] {
        self.list.as_slice()
    }
}
