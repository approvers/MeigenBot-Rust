use crate::db::{MeigenDatabase, MeigenEntry, RegisteredMeigen};
use crate::make_error_enum;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDB {
    #[serde(skip)]
    path: String,

    current_id: usize,
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
    pub fn load(path: &str) -> Result<Self, FileDBError> {
        let file = File::open(path).map_err(FileDBError::open)?;

        let mut result: Self = serde_yaml::from_reader(&file).map_err(FileDBError::deserialize)?;
        result.path = path.into();

        Ok(result)
    }

    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
            current_id: 0,
            meigens: vec![],
            blacklist: vec![],
        }
    }

    pub fn save(&self) -> Result<(), FileDBError> {
        let serialized = serde_yaml::to_string(self).map_err(FileDBError::serialize)?;

        let path = Path::new(&self.path);
        if path.exists() {
            fs::remove_file(path).map_err(FileDBError::delete)?;
        }

        let file = File::create(path).map_err(FileDBError::create)?;

        let mut writer = BufWriter::new(file);
        write!(writer, "{}", serialized).map_err(FileDBError::save)
    }
}

impl MeigenDatabase for FileDB {
    type Error = FileDBError;

    fn save_meigen(&mut self, entry: MeigenEntry) -> Result<&RegisteredMeigen, Self::Error> {
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

    fn meigens(&self) -> &[RegisteredMeigen] {
        self.meigens.as_slice()
    }

    fn delete_meigen(&mut self, id: usize) -> Result<(), Self::Error> {
        let index = self
            .meigens
            .iter()
            .position(|x| x.id == id)
            .ok_or_else(|| FileDBError::nf(id))?;

        self.meigens.remove(index);
        self.save()
    }
}
