use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use std::sync::Arc;
use std::sync::RwLock;

pub async fn status(db: &Arc<RwLock<impl MeigenDatabase>>) -> Result {
    let meigens = db
        .read()
        .unwrap()
        .meigens()
        .await
        .map_err(Error::load_failed)?;

    let text = format!("合計名言数: {}個", meigens.len());

    Ok(text)
}
