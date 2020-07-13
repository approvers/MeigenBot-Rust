use crate::commands::Error;
use crate::commands::Result;
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use std::sync::Arc;
use std::sync::RwLock;

pub async fn delete(db: &Arc<RwLock<impl MeigenDatabase>>, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message
        .args
        .get(0)
        .unwrap()
        .parse()
        .map_err(|e| Error::arg_num_parse_fail(1, e))?;

    db.write()
        .unwrap()
        .delete_meigen(id)
        .await
        .map_err(Error::save_failed)?;

    Ok("削除しました".into())
}
