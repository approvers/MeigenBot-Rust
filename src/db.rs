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
pub trait Database {
    type Error: std::fmt::Display;

    // 名言を保存する。
    fn save_meigen(&mut self, _: MeigenEntry) -> Result<&RegisteredMeigen, Self::Error>;

    // 名言スライスを返す。
    fn meigens(&self) -> &[RegisteredMeigen];

    // 名言を削除する。
    fn delete_meigen(&mut self, id: usize) -> Result<(), Self::Error>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredMeigen {
    id: usize,
    author: String,
    content: String,
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

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    fn _internal_format(id: usize, author: &str, content: &str) -> String {
        format!(
            "Meigen No.{}\n```\n{}\n    --- {}\n```",
            id, content, author
        )
    }

    pub fn format(&self) -> String {
        Self::_internal_format(self.id, &self.author, &self.content())
    }

    pub fn tidy_format(&self, max_length: usize) -> String {
        const BASE: &str = "Meigen No.\n```\n\n    --- \n```";
        const TIDY_SUFFIX: &str = "...";
        const NO_SPACE_MSG: &str = "スペースが足りない...";

        let remain_length = (max_length as i32)
            - (BASE.chars().count() as i32)
            - (self.author.chars().count() as i32);

        // 十分なスペースがあるなら、そのままフォーマットして返す
        if remain_length >= self.content().chars().count() as i32 {
            return self.format();
        }

        // 作者名が長すぎるなどの理由で、...を使った省略でも入らない場合は、NO_SPACE_MSGを突っ込む
        if remain_length <= NO_SPACE_MSG.chars().count() as i32 {
            return format!("Meigen No.{}\n```{}```", self.id, NO_SPACE_MSG);
        }

        // 上記どれにも引っかからない場合、最後を...で削って文字列を減らして返す
        let content = self
            .content()
            .chars()
            .take((remain_length - TIDY_SUFFIX.len() as i32) as usize)
            .chain(TIDY_SUFFIX.chars())
            .collect::<String>();

        Self::_internal_format(self.id, &self.author, &content)
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
