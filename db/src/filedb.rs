use crate::make_error_enum;
use crate::{MeigenDatabase, MeigenEntry, RegisteredMeigen};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::{self, File};
use tokio::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDB {
    #[serde(skip)]
    path: String,

    current_id: u32,
    meigens: Vec<RegisteredMeigen>,
    blacklist: Vec<String>,
}

make_error_enum! {
    FileDBError;
    SaveConfigFailed save(e) => "保存に失敗しました: {}",
    OpenConfigFailed open(e) => "ファイルを開けませんでした: {}",
    CreateConfigFailed create(e) => "ファイル作成に失敗しました: {}",
    DeleteConfigFailed delete(e) => "ファイル削除に失敗しました: {}",

    ConfigAlreadyExist already_exist() => "Configファイルがすでに存在します",
    MeigenNotFound nf(id) => "ID{}を持つ名言は存在しません",

    SerializeFailed serialize(e) => "Serializeに失敗しました: {}",
    DeserializeFailed deserialize(e) => "Deserializeに失敗しました: {}",
}

impl FileDB {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            current_id: 0,
            meigens: vec![],
            blacklist: vec![],
        }
    }

    pub async fn load(path: &str) -> Result<Self, FileDBError> {
        let mut file = File::open(path).await.map_err(FileDBError::open)?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .await
            .map_err(FileDBError::open)?;

        let mut deserialized: FileDB =
            serde_yaml::from_str(&content).map_err(FileDBError::deserialize)?;
        deserialized.path = path.into();

        Ok(deserialized)
    }

    pub async fn save(&self) -> Result<(), FileDBError> {
        let serialized = serde_yaml::to_string(self).map_err(FileDBError::serialize)?;

        let path = Path::new(&self.path);
        if path.exists() {
            fs::remove_file(path).await.map_err(FileDBError::delete)?;
        }

        File::create(path)
            .await
            .map_err(FileDBError::create)?
            .write_all(serialized.as_bytes())
            .await
            .map_err(FileDBError::save)
    }
}

#[async_trait]
impl MeigenDatabase for FileDB {
    type Error = FileDBError;

    // 名言を保存する
    async fn save_meigen(&mut self, entry: MeigenEntry) -> Result<RegisteredMeigen, Self::Error> {
        self.current_id += 1;

        let register_entry = RegisteredMeigen {
            id: self.current_id,
            author: entry.author,
            content: entry.content,
        };

        self.meigens.push(register_entry.clone());
        self.save().await?;

        Ok(register_entry)
    }

    // 名言を削除する
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error> {
        let index = self
            .meigens
            .iter()
            .position(|x| x.id == id)
            .ok_or_else(|| FileDBError::nf(id))?;

        self.meigens.remove(index);
        self.save().await
    }

    // 作者名から名言検索
    async fn search_by_author(&self, author: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        let list = self
            .meigens
            .iter()
            .filter(|x| x.author.contains(author))
            .cloned()
            .collect();
        Ok(list)
    }

    // 名言本体から名言検索
    async fn search_by_content(&self, content: &str) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        let list = self
            .meigens
            .iter()
            .filter(|x| x.content.contains(content))
            .cloned()
            .collect();
        Ok(list)
    }

    // idから名言取得
    async fn get_by_id(&self, id: u32) -> Result<Option<RegisteredMeigen>, Self::Error> {
        Ok(self.meigens.iter().find(|x| x.id == id).cloned())
    }

    // idから名言取得(複数指定) 一致するIDの名言がなかった場合はスキップする
    async fn get_by_ids(&self, ids: &[u32]) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        let mut result = vec![];

        for target_id in ids {
            if let Some(meigen) = self.meigens.iter().find(|x| x.id == *target_id) {
                result.push(meigen.clone())
            }
        }

        Ok(result)
    }

    //len
    async fn len(&self) -> Result<u64, Self::Error> {
        Ok(self.meigens.len() as u64)
    }

    // 現在登録されている名言のなかで一番IDが大きいもの(=現在の(最大)名言ID)を返す
    async fn current_meigen_id(&self) -> Result<u32, Self::Error> {
        Ok(self.meigens.iter().max_by_key(|x| x.id).unwrap().id)
    }

    // 全名言取得
    async fn get_all_meigen(&self) -> Result<Vec<RegisteredMeigen>, Self::Error> {
        Ok(self.meigens.clone())
    }
}
