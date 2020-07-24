use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::ops::Range;

pub mod filedb;
pub mod mongodb;

#[async_trait]
pub trait MeigenDatabase: Send + Sync + Clone + 'static {
    type Error: Display + Debug;

    // 名言を保存する
    async fn save_meigen(&mut self, _: MeigenEntry) -> Result<RegisteredMeigen, Self::Error>;

    // 名言を削除する
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error>;

    // 作者名から名言検索
    async fn search_by_author(&self, author: &str) -> Result<Vec<RegisteredMeigen>, Self::Error>;

    // 名言本体から名言検索
    async fn search_by_content(&self, content: &str) -> Result<Vec<RegisteredMeigen>, Self::Error>;

    // idから名言取得
    async fn get_by_id(&self, id: u32) -> Result<RegisteredMeigen, Self::Error>;

    // idから名言取得(範囲指定)
    async fn get_by_id_range(&self, range: &[u32]) -> Result<RegisteredMeigen, Self::Error>;

    // len
    async fn len(&self) -> Result<u64, Self::Error>;

    // 全名言取得
    async fn get_all_meigen(&self) -> Result<Vec<RegisteredMeigen>, Self::Error>;
}

#[readonly::make]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredMeigen {
    pub id: u32,
    pub author: String,
    pub content: String,
}

#[derive(Debug)]
pub struct MeigenEntry {
    author: String,
    content: String,
}

impl RegisteredMeigen {
    fn from_entry(entry: MeigenEntry, id: u32) -> Self {
        Self {
            id,
            author: entry.author,
            content: entry.content,
        }
    }
}

pub enum MeigenError {
    TooLongMeigen { actual_size: usize, limit: usize },
}

impl MeigenEntry {
    pub fn new(author: String, content: String) -> Result<MeigenEntry, MeigenError> {
        const MEIGEN_MAX_LENGTH: usize = 300;

        let meigen_length = author.chars().count() + content.chars().count();

        if meigen_length > MEIGEN_MAX_LENGTH {
            let err = MeigenError::TooLongMeigen {
                actual_size: meigen_length,
                limit: MEIGEN_MAX_LENGTH,
            };

            return Err(err);
        }

        let result = Self { author, content };
        Ok(result)
    }
}
