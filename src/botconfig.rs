use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

const CONF_FILE_NAME: &str = "./conf.yaml";
const NEW_CONF_FILE_NAME: &str = "./conf.new.yaml";
#[derive(Debug, Serialize, Deserialize)]
pub struct BotConfig {
    pub discord_token: String,
    pub max_meigen_length: u128,
    pub current_id: u128,
    pub meigens: Vec<RegisteredEntry>,
    pub blacklist: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisteredEntry {
    id: u128,
    author: String,
    content: String,
}

#[derive(Debug)]
pub struct MeigenEntry {
    author: String,
    content: String,
}

#[derive(Debug)]
pub struct SaveConfigError(String);
pub struct TooLongMeigenError(String);

impl BotConfig {
    pub fn load() -> Self {
        let path = Path::new(CONF_FILE_NAME);

        if !path.exists() {
            println!("Config file not found");
            Self::create_new_conf(NEW_CONF_FILE_NAME);
            panic!();
        }

        let file = File::open(path).unwrap();

        let deserialize_result: Result<Self, _> = serde_yaml::from_reader(&file);

        if let Err(e) = deserialize_result {
            println!("Parse conf failed: {}", e);
            Self::create_new_conf(NEW_CONF_FILE_NAME);
            panic!();
        }

        deserialize_result.unwrap()
    }

    fn create_new_conf(path_str: &str) {
        let path = Path::new(path_str);
        if path.exists() {
            println!("New conf file exists. To create new one, just delete it.");
            return;
        }

        println!(
            "I create new file, so please fill token, rename to {} and restart.",
            CONF_FILE_NAME
        );

        let new_conf = Self {
            discord_token: "TOKEN HERE".into(),
            max_meigen_length: 300,
            current_id: 0,
            meigens: vec![],
            blacklist: vec![],
        };

        let file = File::create(path).unwrap();
        let mut writer = BufWriter::new(file);

        write!(writer, "{}", serde_yaml::to_string(&new_conf).unwrap()).unwrap();
    }

    pub fn push_new_meigen(
        &mut self,
        entry: MeigenEntry,
    ) -> Result<&RegisteredEntry, SaveConfigError> {
        self.current_id += 1;

        let register_entry = RegisteredEntry {
            id: self.current_id,
            author: entry.author,
            content: entry.content,
        };

        self.meigens.push(register_entry);
        self.save()?;

        Ok(self.meigens.iter().last().unwrap())
    }

    fn save(&self) -> Result<(), SaveConfigError> {
        let serialized = serde_yaml::to_string(self)
            .map_err(|e| SaveConfigError(format!("Serialize failed: {}", e)))?;

        #[inline]
        fn failed(content: &str, e: io::Error) -> SaveConfigError {
            let message = format!(
                "Create file failed: {}.
            Save this content insted of me.
            {}",
                e, content
            );

            SaveConfigError(message)
        }

        let path = Path::new(CONF_FILE_NAME);
        if path.exists() {
            fs::remove_file(path).map_err(|e| failed(&serialized, e))?;
        }

        let file = File::create(path).map_err(|e| failed(&serialized, e))?;

        let mut writer = BufWriter::new(file);
        write!(writer, "{}", serialized).map_err(|e| failed(&serialized, e))?;

        Ok(())
    }
}

impl RegisteredEntry {
    pub fn from_entry(entry: MeigenEntry, id: u128) -> Self {
        Self {
            id,
            author: entry.author,
            content: entry.content,
        }
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
        format!(
            "Meigen No.{}\n```\n{}\n    --- {}\n```",
            &self.id, &self.content, &self.author
        )
    }
}

impl MeigenEntry {
    pub fn new(
        author: String,
        content: String,
        max_length: u128,
    ) -> Result<MeigenEntry, TooLongMeigenError> {
        let meigen_length = author.chars().count() + content.chars().count();

        if meigen_length as u128 >= max_length {
            let err_text = format!(
                "流石に{}文字は長過ぎませんの...? せめて{}文字未満にしてくださいまし...",
                meigen_length, max_length
            );

            return Err(TooLongMeigenError(err_text));
        }

        let result = Self { author, content };
        Ok(result)
    }
}

impl SaveConfigError {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl TooLongMeigenError {
    pub fn into_string(self) -> String {
        self.0
    }
}

impl Display for SaveConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
