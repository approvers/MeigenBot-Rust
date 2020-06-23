use crate::commands::listify;
use crate::commands::{Error, Result};
use crate::db::Database;
use crate::db::RegisteredMeigen;
use crate::message_parser::ParsedMessage;
use std::str::FromStr;

const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

pub fn list(db: &impl Database, message: ParsedMessage) -> Result {
    // 表示する数
    let show_count = message
        .args
        .get(0)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_COUNT), |x| x.parse())
        .map_err(|x| Error::arg_num_parse_fail(1, x))?;

    let page = message
        .args
        .get(1)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_PAGE), |x| x.parse())
        .map_err(|x| Error::arg_num_parse_fail(2, x))?;

    let meigens = db.meigens().iter().collect::<Vec<&RegisteredMeigen>>();

    let result = listify(meigens.as_slice(), show_count, page)?;
    Ok(result)
}

#[inline]
fn parse_or<V: FromStr>(
    default: V,
    text: Option<&String>,
) -> std::result::Result<V, <V as FromStr>::Err> {
    match text {
        Some(num) => num.parse(),
        None => Ok(default),
    }
}
