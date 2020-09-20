use crate::commands::{meigen_tidy_format, Error, Result};
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use std::sync::Arc;
use tokio::sync::RwLock;

const LIST_MEIGEN_DEFAULT_COUNT: i64 = 5;
const LIST_MEIGEN_DEFAULT_PAGE: i64 = 1;

pub async fn list(db: &Arc<RwLock<impl MeigenDatabase>>, message: ParsedMessage) -> Result {
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

    listify(&db, show_count, page).await
}

async fn listify(db: &Arc<RwLock<impl MeigenDatabase>>, show_count: i64, page: i64) -> Result {
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let range = {
        use std::convert::TryInto;

        let meigens_end_index = db
            .read()
            .await
            .current_meigen_id()
            .await
            .map_err(Error::load_failed)? as i64
            + 1;

        if meigens_end_index > show_count {
            let from: usize = {
                (meigens_end_index - show_count - (show_count * (page - 1)))
                    .try_into()
                    .map_err(Error::num_parse_fail)?
            };

            let to: usize = {
                (meigens_end_index - (show_count * (page - 1)))
                    .try_into()
                    .map_err(Error::num_parse_fail)?
            };

            from..to
        } else {
            0..(meigens_end_index as usize)
        }
    };

    let mut indexes = vec![];
    for x in range {
        indexes.push(x as u32);
    }

    let mut result = String::new();
    let meigens = db
        .read()
        .await
        .get_by_ids(&indexes)
        .await
        .map_err(Error::load_failed)?;

    for meigen in &meigens {
        let formatted = meigen_tidy_format(meigen, MAX_LENGTH_PER_MEIGEN);
        result += &format!("\n{}", &formatted);
    }

    if result.is_empty() {
        return Err(Error::no_meigen_matches());
    }

    if result.chars().count() >= LIST_MAX_LENGTH {
        return Err(Error::too_many_meigen_matches());
    }

    Ok(result)
}
