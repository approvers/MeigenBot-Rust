use crate::commands::meigen_format;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use std::sync::Arc;
use std::sync::RwLock;

pub async fn id(db: &Arc<RwLock<impl MeigenDatabase>>, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message.args[0]
        .parse::<u32>()
        .map_err(|e| Error::arg_num_parse_fail(1, e))?;

    let found_meigen = db
        .read()
        .unwrap()
        .get_by_id(id)
        .await
        .map_err(Error::load_failed)?;

    Ok(meigen_format(&found_meigen))
}
