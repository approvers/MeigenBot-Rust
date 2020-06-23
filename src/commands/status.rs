use crate::commands::Result;
use crate::db::Database;

pub fn status(db: &impl Database) -> Result {
    let text = format!("合計名言数: {}個", db.meigens().len());
    Ok(text)
}
