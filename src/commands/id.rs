use crate::commands::meigen_format;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

pub async fn id(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message.args[0]
        .parse::<u32>()
        .map_err(|e| Error::arg_num_parse_fail(0, e))?;

    let found_meigen = db
        .meigens()
        .await
        .iter()
        .find(|x| x.id == id)
        .ok_or_else(|| Error::meigen_nf(id))?;

    Ok(meigen_format(found_meigen))
}
