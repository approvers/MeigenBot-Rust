use crate::commands::format;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

pub fn id(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message.args[0]
        .parse::<usize>()
        .map_err(|e| Error::arg_num_parse_fail(0, e))?;

    let found_meigen = db
        .meigens()
        .iter()
        .find(|x| x.id == id)
        .ok_or(Error::meigen_nf(id))?;

    Ok(format(found_meigen))
}
