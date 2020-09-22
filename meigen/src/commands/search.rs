mod author;
mod content;
mod help;

pub(crate) use author::author;
pub(crate) use content::content;
pub(crate) use help::help;

use crate::commands::meigen_tidy_format;
use crate::db::{MeigenDatabase, RegisteredMeigen};
use crate::message_parser::ParsedMessage;
use crate::CommandResult;
use crate::Error;
use std::sync::Arc;
use tokio::sync::RwLock;

const AUTHOR_SEARCH_COMMAND: &str = "author";
const WORD_SEARCH_COMMAND: &str = "content";
const SEARCH_HELP_COMMAND: &str = "help";

pub(crate) async fn search<D>(db: &Arc<RwLock<D>>, message: ParsedMessage) -> CommandResult
where
    D: MeigenDatabase,
{
    const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
    const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

    if message.args.len() <= 1 {
        return help::<D>();
    }

    let sub_command = &message.args[0];
    let search_query = &message.args[1];

    let show_count = message
        .args
        .get(2)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_COUNT), |x| x.parse())
        .map_err(|x| Error::NumberParseFail {
            args_index: 2,
            source: x,
        })?;

    let page = message
        .args
        .get(3)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_PAGE), |x| x.parse())
        .map_err(|x| Error::NumberParseFail {
            args_index: 3,
            source: x,
        })?;

    match sub_command.as_str() {
        AUTHOR_SEARCH_COMMAND => author(db, search_query, show_count, page).await,
        WORD_SEARCH_COMMAND => content(db, search_query, show_count, page).await,
        SEARCH_HELP_COMMAND => help::<D>(),
        _ => Err(Error::InvalidSearchSubCommand),
    }
}

fn listify(slice: &[RegisteredMeigen], show_count: i32, page: i32) -> CommandResult {
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let range = {
        use std::convert::TryInto;

        let meigens_end_index = slice.len() as i32;
        if meigens_end_index > show_count {
            let from: usize = {
                (meigens_end_index - show_count - (show_count * (page - 1)))
                    .try_into()
                    .map_err(|_| Error::ArgsTooBigNumber)?
            };

            let to: usize = {
                (meigens_end_index - (show_count * (page - 1)))
                    .try_into()
                    .map_err(|_| Error::ArgsTooBigNumber)?
            };

            &slice[from..to]
        } else {
            &slice[0..(slice.len())]
        }
    };

    let mut result = String::new();

    for meigen in range {
        let formatted = meigen_tidy_format(meigen, MAX_LENGTH_PER_MEIGEN);
        result += &format!("\n{}", &formatted);
    }

    if result.is_empty() {
        return Err(Error::NoMeigenHit);
    }

    if result.chars().count() >= LIST_MAX_LENGTH {
        return Err(Error::TooManyMeigenHit);
    }

    Ok(result)
}
