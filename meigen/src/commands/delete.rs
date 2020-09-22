use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn delete<D>(db: &Arc<RwLock<D>>, message: ParsedMessage) -> CommandResult
where
    D: MeigenDatabase,
{
    if message.args.is_empty() {
        return Err(Error::NotEnoughArgs);
    }

    let id = message
        .args
        .get(0)
        .unwrap()
        .parse()
        .map_err(|e| Error::NumberParseFail {
            args_index: 1,
            source: e,
        })?;

    db.write()
        .await
        .delete_meigen(id)
        .await
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    Ok("削除しました".into())
}
