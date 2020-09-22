use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use crate::Error;
use chrono::prelude::*;
use chrono_tz::Asia::Tokyo;
use interface::FileEntry;
use std::sync::Arc;
use tokio::sync::RwLock;

enum ExportFormat {
    Json,
    Yaml,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Yaml => write!(f, "yaml"),
        }
    }
}

pub(crate) struct MessageWithFile {
    pub(crate) msg: String,
    pub(crate) file: FileEntry,
}

pub(crate) async fn export<D>(
    db: &Arc<RwLock<D>>,
    message: ParsedMessage,
) -> Result<MessageWithFile, Error>
where
    D: MeigenDatabase,
{
    let mut mode = ExportFormat::Json;

    if message.args.len() >= 1 {
        match message.args[0].to_lowercase().as_str() {
            "json" => mode = ExportFormat::Json,
            "yaml" => mode = ExportFormat::Yaml,

            _ => return Err(Error::InvalidExportFormat),
        }
    }

    let entries = db
        .read()
        .await
        .get_all_meigen()
        .await
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    let text = {
        match mode {
            #[rustfmt::skip]
            ExportFormat::Json => {
                serde_json::to_string_pretty(&entries).map_err(|x| Error::ExportError(Box::new(x)))?
            },

            ExportFormat::Yaml => {
                serde_yaml::to_string(&entries).map_err(|x| Error::ExportError(Box::new(x)))?
            }
        }
    };

    let utc = Utc::now();
    let utc = NaiveDateTime::from_timestamp(utc.timestamp(), utc.timestamp_subsec_nanos());
    let jst = Tokyo.from_utc_datetime(&utc);

    let file_name = match mode {
        ExportFormat::Json => format!("meigen-entries-{}.json", jst.format("%Y%m%d-%H%M%S")),
        ExportFormat::Yaml => format!("meigen-entries-{}.yaml", jst.format("%Y%m%d-%H%M%S"),),
    };

    let result = MessageWithFile {
        msg: format!("{}件の名言を{}形式で書き出しました", entries.len(), mode),
        file: FileEntry {
            name: file_name,
            data: text.into_bytes(),
        },
    };

    Ok(result)
}
