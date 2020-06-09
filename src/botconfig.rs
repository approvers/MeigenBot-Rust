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
    pub bot_entries: Vec<BotEntry>,
    pub meigens: Vec<MeigenEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BotEntry {
    pub bot_id: u64,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeigenEntry {
    pub author: String,
    pub content: String,
}

#[derive(Debug)]
pub struct Error(String);

impl BotConfig {
    pub fn load() -> Self {
        let path = Path::new(CONF_FILE_NAME);
        let file = if !path.exists() {
            println!("config file not found");
            Self::create_new_conf();

            panic!("Panic due to above error")
        } else {
            File::open(path).unwrap()
        };

        let deserialize_result: Result<Self, serde_yaml::Error> = serde_yaml::from_reader(&file);

        if let Err(e) = deserialize_result {
            println!("Parse conf failed: {}", e);
            Self::create_new_conf();

            panic!("Panic due to above error");
        } else {
            deserialize_result.unwrap()
        }
    }

    fn create_new_conf() {
        let path = Path::new(NEW_CONF_FILE_NAME);
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
            bot_entries: vec![],
            meigens: vec![],
        };

        {
            let file = File::create(NEW_CONF_FILE_NAME).unwrap();
            let mut writer = BufWriter::new(file);

            write!(writer, "{}", serde_yaml::to_string(&new_conf).unwrap()).unwrap();
        }
    }

    pub fn new_meigen(&mut self, entry: MeigenEntry) -> Result<(), Error> {
        self.meigens.push(entry);
        self.save()?;

        Ok(())
    }

    fn save(&self) -> Result<(), Error> {
        let serialized =
            serde_yaml::to_string(self).map_err(|e| Error(format!("Serialize failed: {}", e)))?;

        #[inline]
        fn failed(content: &str, e: io::Error) -> Error {
            let message = format!(
                "Create file failed: {}.
            Save this content insted of me.
            {}",
                e, content
            );

            Error(message)
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

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
