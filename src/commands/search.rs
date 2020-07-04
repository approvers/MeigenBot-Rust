mod author;
mod help;
mod word;

pub use author::author;
pub use word::content;
pub use help::help;

use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

const AUTHOR_SEARCH_COMMAND: &str = "author";
const WORD_SEARCH_COMMAND: &str = "word";
const SEARCH_HELP_COMMAND: &str = "help";

pub fn search(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
    const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
    const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

    if message.args.is_empty() {
        return help();
    }

    let sub_command = &message.args[0];
    let search_query = &message.args[1];

    let show_count = message
        .args
        .get(2)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_COUNT), |x| x.parse())
        .map_err(Error::num_parse_fail)?;

    let page = message
        .args
        .get(3)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_PAGE), |x| x.parse())
        .map_err(Error::num_parse_fail)?;

    match sub_command.as_str() {
        AUTHOR_SEARCH_COMMAND => author(db, search_query, show_count, page),
        WORD_SEARCH_COMMAND => content(db, search_query, show_count, page),
        SEARCH_HELP_COMMAND => help(),
        _ => Err(Error::invalid_search_subcommand())
    }

}
