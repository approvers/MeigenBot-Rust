use crate::commands::{Error, Result};
use crate::message_parser::ParsedMessage;
use db::{MeigenDatabase, RegisteredMeigen};
use rand::Rng;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn random(db: &Arc<RwLock<impl MeigenDatabase>>, message: ParsedMessage) -> Result {
    let count: usize = {
        message
            .args
            .get(0)
            .map_or(Ok(1), |x| x.parse())
            .map_err(|e| Error::arg_num_parse_fail(1, e))?
    };

    let meigen_count = db
        .read()
        .await
        .current_meigen_id()
        .await
        .map_err(Error::load_failed)? as u32;

    let rands = gen_rand_vec(count, 0, meigen_count);

    let meigens = db
        .read()
        .await
        .get_by_ids(&rands)
        .await
        .map_err(Error::load_failed)?;

    local_listify(&meigens)
}

fn gen_rand_vec(count: usize, range_from: u32, range_to: u32) -> Vec<u32> {
    let mut rng = rand::thread_rng();

    let mut result: Vec<u32> = vec![0; count as usize];
    result
        .iter_mut()
        .for_each(|x| *x = rng.gen_range(range_from, range_to));

    result
}

fn local_listify(list: &[RegisteredMeigen]) -> Result {
    const LIST_MAX_LENGTH: usize = 500;
    const MAX_LENGTH_PER_MEIGEN: usize = 50;

    let mut result = String::new();

    for meigen in list {
        let formatted = crate::commands::meigen_tidy_format(meigen, MAX_LENGTH_PER_MEIGEN);
        result += &format!("{}\n", formatted);
    }

    if result.is_empty() {
        return Err(Error::no_meigen_matches());
    }

    if result.chars().count() >= LIST_MAX_LENGTH {
        return Err(Error::too_many_meigen_matches());
    }

    Ok(result)
}
