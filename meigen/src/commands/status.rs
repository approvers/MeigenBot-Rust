use crate::db::MeigenDatabase;
use crate::CommandResult;
use crate::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn status<D>(db: &Arc<RwLock<D>>) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    let meigen_count = db.read().await.len().await.map_err(Error::DatabaseError)?;

    Ok(format!("合計名言数: {}個", meigen_count))
}
