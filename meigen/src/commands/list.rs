use crate::commands::meigen_tidy_format;
use crate::db::MeigenDatabase;
use crate::message_parser::ParsedMessage;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

const LIST_MEIGEN_DEFAULT_COUNT: i64 = 5;
const LIST_MEIGEN_DEFAULT_PAGE: i64 = 1;

pub(crate) async fn list<D>(db: &Arc<RwLock<D>>, message: ParsedMessage) -> CommandResult
where
    D: MeigenDatabase,
{
    // 表示する数
    let show_count = message
        .args
        .get(0)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_COUNT), |x| x.parse())
        .map_err(|x| Error::NumberParseFail {
            args_index: 1,
            source: x,
        })?;

    let page = message
        .args
        .get(1)
        .map_or(Ok(LIST_MEIGEN_DEFAULT_PAGE), |x| x.parse())
        .map_err(|x| Error::NumberParseFail {
            args_index: 2,
            source: x,
        })?;

    listify(&db, show_count, page).await
}

async fn listify<D>(db: &Arc<RwLock<D>>, show_count: i64, page: i64) -> CommandResult
where
    D: MeigenDatabase,
{
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let range = {
        use std::convert::TryInto;

        let meigens_end_index = db
            .read()
            .await
            .current_meigen_id()
            .await
            .map_err(|x| Error::DatabaseError(Box::new(x)))? as i64
            + 1;

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
        .map_err(|x| Error::DatabaseError(Box::new(x)))?;

    for meigen in &meigens {
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
