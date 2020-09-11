use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn status(db: &Arc<RwLock<impl MeigenDatabase>>) -> Result {
    let meigen_count = db.read().await.len().await.map_err(Error::load_failed)?;

    Ok(format!("合計名言数: {}個", meigen_count))
}
