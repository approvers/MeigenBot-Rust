use crate::commands::{Error, Result};
use crate::db::Database;
use crate::message_parser::ParsedMessage;

pub fn id(db: &impl Database, message: ParsedMessage) -> Result {
    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let id = message.args[0]
        .parse::<usize>()
        .map_err(|e| Error::arg_num_parse_fail(0, e))?;

    match db.meigens().iter().find(|x| x.id() == id) {
        Some(meigen) => Ok(meigen.format()),
        None => Err(Error::meigen_nf(id)),
    }
}
