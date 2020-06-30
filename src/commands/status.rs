use crate::commands::Result;
use crate::db::MeigenDatabase;

pub fn status(db: &impl MeigenDatabase) -> Result {
    let text = format!("合計名言数: {}個", db.meigens().len());
    Ok(text)
}
