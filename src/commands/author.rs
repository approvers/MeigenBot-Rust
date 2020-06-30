use crate::commands::listify;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;
use crate::message_parser::ParsedMessage;

pub fn author(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
    const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
    const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

    if message.args.is_empty() {
        return Err(Error::not_enough_args());
    }

    let target_author = &message.args[0];

    let show_count = message
        .args
        .get(1)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_COUNT), |x| x.parse())
        .map_err(Error::num_parse_fail)?;

    let page = message
        .args
        .get(2)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_PAGE), |x| x.parse())
        .map_err(Error::num_parse_fail)?;

    let filtered = db
        .meigens()
        .iter()
        .filter(|x| x.author.contains(target_author))
        .collect::<Vec<&RegisteredMeigen>>();

    listify(filtered.as_slice(), show_count, page)
}
