pub mod auth;
pub mod warp;

use anyhow::{Context as _, Result};
use serde::Deserialize;
use tokio_stream::StreamExt;

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

    if count > MAX_FETCH_COUNT {
        return Err(CustomError::FetchLimitExceeded.into());
    }

    let max = db
        .read()
        .await
        .get_current_id()
        .await
        .context("failed to get current id")
        .map_err(CustomError::Internal)?;

    let mut list = async_stream::try_stream! {
        use rand::prelude::*;
        let mut rng = StdRng::from_rng(&mut rand::thread_rng()).unwrap();
        loop {
            let pos = rng.gen_range(1..=max);

            if let Some(m) = db.read().await.load(pos).await.context("failed to fetch meigen")? {
                yield m;
            }
        }
    }
    .take(count)
    .collect::<Result<Vec<_>, anyhow::Error>>()
    .await
    .context("failed to fetch stream")
    .map_err(CustomError::Internal)?;

    list.sort_unstable_by_key(|x| x.id);

    Ok(list)
}

#[derive(Deserialize)]
struct FindRequest {
    offset: Option<u32>,
    limit: Option<u8>,
    author: Option<String>,
    content: Option<String>,
}

async fn search(
    body: FindRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<Vec<Meigen>, CustomError> {
    let limit = body.limit.unwrap_or(5);
    let offset = body.offset.unwrap_or(0);

    if limit as usize > MAX_FETCH_COUNT {
        return Err(CustomError::FetchLimitExceeded.into());
    }

    if body
        .author
        .as_ref()
        .map(|x| x.chars().count() > SEARCH_STRING_LENGTH_LIMIT)
        .unwrap_or(false)
    {
        return Err(CustomError::SearchWordLengthLimitExceeded.into());
    }

    if body
        .content
        .as_ref()
        .map(|x| x.chars().count() > SEARCH_STRING_LENGTH_LIMIT)
        .unwrap_or(false)
    {
        return Err(CustomError::SearchWordLengthLimitExceeded.into());
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
            return Err(CustomError::TooBigOffset.into());
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
