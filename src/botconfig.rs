use log::trace;
use serde::{Deserialize, Serialize};

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

macro_rules! make_error_enum {
    ($enum_name:ident; $($variant:ident => $description:expr),+ $(,)?) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($variant,)+
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let desc = match *self {
                    $($variant => $description,)+
                };
                write!(f, "{}", desc)
            }
        }
    };

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

#[derive(Debug, Serialize, Deserialize)]
pub struct BotConfig {
    #[serde(skip)]
    path: String,

    pub discord_token: String,
    pub max_meigen_length: u128,
    pub current_id: u128,
    pub meigens: Vec<RegisteredMeigen>,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredMeigen {
    id: u128,
    author: String,
    content: String,
}

#[derive(Debug)]
pub struct MeigenEntry {
    author: String,
    content: String,
}

make_error_enum! {
    ConfigError;
    SaveConfigFailed save(e) => "保存に失敗しました: {}",
    OpenConfigFailed open(e) => "ファイルを開けませんでした: {}",
    CreateConfigFailed create(e) => "ファイル作成に失敗しました: {}",
    DeleteConfigFailed delete(e) => "ファイル削除に失敗しました: {}",

    ConfigAlreadyExist already_exist() => "Configファイルがすでに存在します",

    SerializeFailed serialize(e) => "Seralizeに失敗しました: {}",
    DeserializeFailed deserialize(e) => "Deserializeに失敗しました: {}",
}

impl BotConfig {
    pub fn load(path: &str) -> Result<Self, ConfigError> {
        let file = File::open(path).map_err(|x| ConfigError::open(x))?;

        let mut result: Self = serde_yaml::from_reader(&file).map_err(ConfigError::deserialize)?;
        result.path = path.into();

        Ok(result)
    }

    pub fn create_new_conf(path_str: &str) -> Result<(), ConfigError> {
        let path = Path::new(path_str);
        if path.exists() {
            return Err(ConfigError::already_exist());
        }

        let new_conf = Self {
            path: path_str.into(),
            discord_token: "TOKEN HERE".into(),
            max_meigen_length: 300,
            current_id: 0,
            meigens: vec![],
            blacklist: vec![],
        };

        let serialized = serde_yaml::to_string(&new_conf).map_err(ConfigError::serialize)?;

        let file = File::create(path).map_err(ConfigError::create)?;
        let mut writer = BufWriter::new(file);

        write!(writer, "{}", serialized).map_err(ConfigError::save)
    }

    pub fn push_new_meigen(
        &mut self,
        entry: MeigenEntry,
    ) -> Result<&RegisteredMeigen, ConfigError> {
        self.current_id += 1;

        let register_entry = RegisteredMeigen {
            id: self.current_id,
            author: entry.author,
            content: entry.content,
        };

        self.meigens.push(register_entry);
        self.save()?;

        Ok(self.meigens.iter().last().unwrap())
    }

    fn save(&self) -> Result<(), ConfigError> {
        let serialized = serde_yaml::to_string(self).map_err(ConfigError::serialize)?;

        let path = Path::new(&self.path);
        if path.exists() {
            fs::remove_file(path).map_err(ConfigError::delete)?;
        }

        let file = File::create(path).map_err(ConfigError::create)?;

        let mut writer = BufWriter::new(file);
        write!(writer, "{}", serialized).map_err(ConfigError::save)
    }
}

impl RegisteredMeigen {
    fn from_entry(entry: MeigenEntry, id: u128) -> Self {
        Self {
            id,
            author: entry.author,
            content: entry.content,
        }
    }

    fn internal_format(id: u128, author: &str, content: &str) -> String {
        format!(
            "Meigen No.{}\n```\n{}\n    --- {}\n```",
            id, content, author
        )
    }

    pub fn id(&self) -> u128 {
        self.id
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn format(&self) -> String {
        Self::internal_format(self.id, &self.author, &self.content())
    }

    pub fn tidy_format(&self, max_length: usize) -> String {
        const BASE: &str = "Meigen No.\n```\n\n    --- \n```";
        const TIDY_SUFFIX: &str = "...";
        const NO_SPACE_MSG: &str = "スペースが足りない...";

        let remain_length = (max_length as i32)
            - (BASE.chars().count() as i32)
            - (self.author.chars().count() as i32);

        if remain_length >= self.content().chars().count() as i32 {
            trace!("Didn't tidied");
            return self.format();
        }

        if remain_length <= NO_SPACE_MSG.chars().count() as i32 {
            trace!("There isn't enough space.");
            return format!("Meigen No.{}\n```{}```", self.id, NO_SPACE_MSG);
        }

        trace!("Tidied with TIDY_SUFFIX");
        let content = self
            .content()
            .chars()
            .take((remain_length - TIDY_SUFFIX.len() as i32) as usize)
            .chain(TIDY_SUFFIX.chars())
            .collect::<String>();

        Self::internal_format(self.id, &self.author, &content)
    }
}

make_error_enum! {
    MeigenError;
    TooLong too_long(passed_length, max_length) => "流石に{}文字は長過ぎませんの...? せめて{}文字未満にしてくださいまし..."
}

impl MeigenEntry {
    pub fn new(
        author: String,
        content: String,
        max_length: u128,
    ) -> Result<MeigenEntry, MeigenError> {
        let meigen_length = author.chars().count() + content.chars().count();

        if meigen_length as u128 >= max_length {
            return Err(MeigenError::too_long(meigen_length, max_length));
        }

        let result = Self { author, content };
        Ok(result)
    }
}
