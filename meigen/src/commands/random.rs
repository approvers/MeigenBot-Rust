use crate::commands::meigen_tidy_format;
use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;
use crate::message_parser::ParsedMessage;
use crate::{CommandResult, Error};
use std::sync::Arc;
use tokio::sync::RwLock;

pub(crate) async fn random<D>(db: &Arc<RwLock<D>>, message: ParsedMessage) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    let count: usize = {
        message
            .args
            .get(0)
            .map_or(Ok(1), |x| x.parse())
            .map_err(|e| Error::NumberParseFail {
                args_index: 1,
                source: e,
            })?
    };

    let meigen_count = db
        .read()
        .await
        .current_meigen_id()
        .await
        .map_err(Error::DatabaseError)? as u32;

    let rands = gen_rand_vec(count, 0, meigen_count);

    let meigens = db
        .read()
        .await
        .get_by_ids(&rands)
        .await
        .map_err(Error::DatabaseError)?;

    local_listify::<D>(&meigens)
}

fn gen_rand_vec(count: usize, range_from: u32, range_to: u32) -> Vec<u32> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let mut result: Vec<u32> = vec![0; count as usize];
    result
        .iter_mut()
        .for_each(|x| *x = rng.gen_range(range_from, range_to));

    result
}

fn local_listify<D>(list: &[RegisteredMeigen]) -> CommandResult<D>
where
    D: MeigenDatabase,
{
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let mut result = String::new();

    for meigen in list {
        let formatted = meigen_tidy_format(meigen, MAX_LENGTH_PER_MEIGEN);
        result += &format!("{}\n", formatted);
    }

    if result.is_empty() {
        return Err(Error::NoMeigenHit);
    }

    if result.chars().count() >= LIST_MAX_LENGTH {
        return Err(Error::TooManyMeigenHit);
    }

    Ok(result)
}
