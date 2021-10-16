pub mod auth;

#[cfg(feature = "api_http")]
pub mod warp;

#[cfg(feature = "api_grpc")]
pub mod grpc;

#[cfg(feature = "api_graphql")]
mod graphql;

use anyhow::{Context as _, Result};
use rand::{prelude::SmallRng, Rng, SeedableRng};
use serde::Deserialize;

use crate::{
    db::{FindOptions, MeigenDatabase},
    model::Meigen,
    Synced,
};

const SEARCH_STRING_LENGTH_LIMIT: usize = 100;
const MAX_FETCH_COUNT: usize = 50;

#[derive(Debug)]
enum CustomError {
    Internal(anyhow::Error),
    Authentication,
    FetchLimitExceeded,
    SearchWordLengthLimitExceeded,
    TooBigOffset,
}

impl CustomError {
    fn describe(&self) -> &'static str {
        match *self {
            CustomError::Internal(_) => "internal server error",
            CustomError::FetchLimitExceeded => "attempted to get too many meigens",
            CustomError::SearchWordLengthLimitExceeded => "search keyword is too long",
            CustomError::TooBigOffset => "offset is too big",
            CustomError::Authentication => "unauthorized",
        }
    }
}

async fn get(id: u32, db: Synced<impl MeigenDatabase>) -> Result<Option<Meigen>, CustomError> {
    db.read()
        .await
        .load(id)
        .await
        .map_err(CustomError::Internal)
}

#[derive(Deserialize)]
struct RandomRequest {
    count: Option<usize>,
}

async fn random(
    body: RandomRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<Vec<Meigen>, CustomError> {
    let count = body.count.unwrap_or(1);
    let max = db
        .read()
        .await
        .get_current_id()
        .await
        .context("failed to get current id")
        .map_err(CustomError::Internal)? as usize;

    if count > MAX_FETCH_COUNT || count > max {
        return Err(CustomError::FetchLimitExceeded);
    }

    let mut rng = SmallRng::from_rng(&mut rand::thread_rng()).unwrap();

    let mut meigens = Vec::<Meigen>::with_capacity(count);

    while meigens.len() != count {
        let want = count - meigens.len();
        let mut try_fetch = Vec::with_capacity(want);

        loop {
            let new_id_candidate = rng.gen_range(1..=max as u32);

            if try_fetch.contains(&new_id_candidate) {
                continue;
            }

            if meigens.iter().any(|x| x.id == new_id_candidate) {
                continue;
            }

            try_fetch.push(new_id_candidate);
            if try_fetch.len() == want {
                break;
            }
        }

        let mut fetched = db
            .read()
            .await
            .load_bulk(&try_fetch)
            .await
            .context("failed to failed to bulk load")
            .map_err(CustomError::Internal)?;

        meigens.append(&mut fetched);
    }

    Ok(meigens)
}

#[derive(Deserialize)]
struct SearchRequest {
    offset: Option<u32>,
    limit: Option<u8>,
    author: Option<String>,
    content: Option<String>,
}

async fn search(
    body: SearchRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<Vec<Meigen>, CustomError> {
    let limit = body.limit.unwrap_or(5);
    let offset = body.offset.unwrap_or(0);

    if limit as usize > MAX_FETCH_COUNT {
        return Err(CustomError::FetchLimitExceeded);
    }

    let check_word_len = |x: &Option<String>| {
        x.as_ref()
            .map(|x| x.chars().count() > SEARCH_STRING_LENGTH_LIMIT)
            .unwrap_or(false)
    };

    if check_word_len(&body.author) {
        return Err(CustomError::SearchWordLengthLimitExceeded);
    }

    if check_word_len(&body.content) {
        return Err(CustomError::SearchWordLengthLimitExceeded);
    }

    if offset > 0 {
        let max = db
            .read()
            .await
            .get_current_id()
            .await
            .context("failed to get current id")
            .map_err(CustomError::Internal)?;

        if offset > max {
            return Err(CustomError::TooBigOffset);
        }
    }

    let list = db
        .read()
        .await
        .find(FindOptions {
            author: body.author.as_ref().map(|x| x as _),
            content: body.content.as_ref().map(|x| x as _),
            offset,
            limit,
        })
        .await
        .context("failed to find")
        .map_err(CustomError::Internal)?;

    Ok(list)
}
