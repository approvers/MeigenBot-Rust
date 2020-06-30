use serde::{Deserialize, Serialize};
pub mod filedb;

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

#[macro_export]
macro_rules! make_error_enum {
    ($enum_name:ident; $($variant:ident $func_name:ident($($($vars:ident),+ $(,)?)?) => $format:expr),+ $(,)?) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($variant(String),)+
        }

        impl $enum_name {
            $ (
                pub fn $func_name($($($vars: impl std::fmt::Display,)*)?) -> $enum_name {
                    $enum_name::$variant(format!($format, $($($vars),+)?))
                }
            )+
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($enum_name::$variant(text) => write!(f, "{}", text),)+
                }
            }
        }
    };
}
pub trait MeigenDatabase {
    type Error: std::fmt::Display;

    // 名言を保存する。
    fn save_meigen(&mut self, _: MeigenEntry) -> Result<&RegisteredMeigen, Self::Error>;

    // 名言スライスを返す。
    fn meigens(&self) -> &[RegisteredMeigen];

    // 名言を削除する。
    fn delete_meigen(&mut self, id: usize) -> Result<(), Self::Error>;
}

#[readonly::make]
#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredMeigen {
    pub id: usize,
    pub author: String,
    pub content: String,
}

#[derive(Debug)]
pub struct MeigenEntry {
    author: String,
    content: String,
}

impl RegisteredMeigen {
    fn from_entry(entry: MeigenEntry, id: usize) -> Self {
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
