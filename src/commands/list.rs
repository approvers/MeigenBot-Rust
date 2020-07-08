use crate::commands::listify;
use crate::commands::{Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;

const LIST_MEIGEN_DEFAULT_COUNT: i32 = 5;
const LIST_MEIGEN_DEFAULT_PAGE: i32 = 1;

pub async fn list(db: &impl MeigenDatabase, message: ParsedMessage) -> Result {
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

    let meigens = db.meigens().await.map_err(Error::load_failed)?;
    let meigen_refs = meigens.iter().collect::<Vec<&_>>();

    listify(meigen_refs.as_slice(), show_count, page)
}
