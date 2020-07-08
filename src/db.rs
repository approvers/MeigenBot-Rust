use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod filedb;
pub mod mongodb;

/// エラーを表すためのenum作成マクロ。
/// Trailing comma 対応
/// # 書式
///
/// ```
/// make_error_enum! {
///     enum_name;
///     variant_name generator_func_name(format_args) => "format_template",
///     // バリアント定義は何個でも書ける
/// }
/// ```

#[async_trait]
pub trait MeigenDatabase: Send + Sync {
    type Error: std::fmt::Display;

    // 名言を保存する。
    async fn save_meigen(&mut self, _: MeigenEntry) -> Result<RegisteredMeigen, Self::Error>;

    // 名言を削除する。
    async fn delete_meigen(&mut self, id: u32) -> Result<(), Self::Error>;

    // 名言スライスを返す。
    async fn meigens(&self) -> Result<Vec<RegisteredMeigen>, Self::Error>;
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
