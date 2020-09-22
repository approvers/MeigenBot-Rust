use crate::commands::meigen_format;
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn id<D>(db: &Arc<RwLock<D>>, message: ParsedMessage) -> CommandResult
where
    D: MeigenDatabase,
{
    if message.args.is_empty() {
        return Err(Error::NotEnoughArgs);
    }

    let id = message.args[0]
        .parse::<u32>()
        .map_err(|e| Error::NumberParseFail {
            args_index: 1,
            source: e,
        })?;

    let found_meigen = db
        .read()
        .await
        .get_by_id(id)
        .await
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    match found_meigen {
        Some(meigen) => Ok(meigen_format(&meigen)),

        None => Err(Error::NoMeigenHit),
    }
}
