pub mod auth;

use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use anyhow::{Context as _, Result};
use auth::Authenticator;
use reqwest::StatusCode;
use serde::{Deserialize};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use warp::{Filter, Rejection};

use crate::{
    db::{FindOptions, MeigenDatabase},
    model::Meigen,
    Synced,
};

const SEARCH_STRING_LENGTH_LIMIT: usize = 100;
const MAX_FETCH_COUNT: usize = 50;

pub struct HttpApiServer<D: MeigenDatabase, A: Authenticator> {
    db: Synced<D>,
    auth: A,
}

impl<D: MeigenDatabase, A: Authenticator> HttpApiServer<D, A> {
    pub fn new(db: D, auth: A) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            auth,
        }
    }

    pub async fn start(self, ip: impl Into<SocketAddr>) {
        let route = warp::body::content_length_limit(3 * 1024)
            .and(
                warp::path!("v1" / u32)
                    .and(warp::post())
                    .and(auth_filter(self.auth.clone()))
                    .and(inject(Arc::clone(&self.db)))
                    .and_then(|a, b| async move { get(a, b).await.map_err(Rejection::from) })
                    .map(|x| warp::reply::json(&x))
                    .or(warp::path!("v1" / "random")
                        .and(warp::post())
                        .and(auth_filter(self.auth.clone()))
                        .and(warp::query::query())
                        .and(inject(Arc::clone(&self.db)))
                        .and_then(|a, b| async { random(a, b).await.map_err(Rejection::from) })
                        .map(|x| warp::reply::json(&x)))
                    .or(warp::path!("v1" / "search")
                        .and(warp::post())
                        .and(auth_filter(self.auth.clone()))
                        .and(warp::query::query())
                        .and(inject(Arc::clone(&self.db)))
                        .and_then(|a, b| async { search(a, b).await.map_err(Rejection::from) })
                        .map(|x| warp::reply::json(&x))),
            )
            .recover(recover)
            .with(warp::trace::request());

        warp::serve(route).bind(ip.into()).await;
    }
}

fn auth_filter<A: Authenticator>(auth: A) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::header::<String>("gauth-token")
        .and(inject(auth))
        .and_then(|token: String, auth: A| async move {
            auth.auth(&token)
                .await
                .map_err(|e| match e {
                    auth::Error::Internal(e) => CustomError::Internal(e),
                    auth::Error::InvalidToken => CustomError::Authentication,
                })
                .map_err(Into::<Rejection>::into)
        })
        .map(|_| ())
        .untuple_one()
}

fn inject<T>(t: T) -> impl Filter<Extract = (T,), Error = Infallible> + Clone
where
    T: Send + Clone,
{
    warp::any().map(move || t.clone())
}

#[derive(Debug)]
enum CustomError {
    Internal(anyhow::Error),
    Authentication,
    NotFound,
    FetchLimitExceeded,
    SearchWordLengthLimitExceeded,
    TooBigOffset,
}

impl warp::reject::Reject for CustomError {}

async fn recover(r: Rejection) -> Result<impl warp::Reply, Rejection> {
    let (code, msg) = match r.find::<CustomError>() {
        None => return Err(r),
        Some(&CustomError::Internal(ref e)) => {
            tracing::error!("internal error: {:#?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }

        Some(&CustomError::Authentication) => (StatusCode::UNAUTHORIZED, "unauthorized"),
        Some(&CustomError::NotFound) => (StatusCode::NOT_FOUND, "not found such meigen"),
        Some(&CustomError::FetchLimitExceeded) => {
            (StatusCode::BAD_REQUEST, "attempted to get too many meigens")
        }
        Some(&CustomError::SearchWordLengthLimitExceeded) => {
            (StatusCode::BAD_REQUEST, "search keyword is too long")
        }
        Some(&CustomError::TooBigOffset) => (StatusCode::BAD_REQUEST, "offset is too big"),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "error": msg })),
        code,
    ))
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
    count: usize,
}

async fn random(
    body: RandomRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<Vec<Meigen>, CustomError> {
    if body.count > MAX_FETCH_COUNT {
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
    .take(body.count)
    .collect::<Result<Vec<_>, anyhow::Error>>()
    .await
    .context("failed to fetch stream")
    .map_err(CustomError::Internal)?;

    list.sort_unstable_by_key(|x| x.id);

    Ok(list)
}

#[derive(Deserialize)]
struct FindRequest {
    offset: u32,
    limit: u8,
    author: Option<String>,
    content: Option<String>,
}

async fn search(
    body: FindRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<Vec<Meigen>, CustomError> {
    if body.limit as usize > MAX_FETCH_COUNT {
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

    let max = db
        .read()
        .await
        .get_current_id()
        .await
        .context("failed to get current id")
        .map_err(CustomError::Internal)?;

    if body.offset > max {
        return Err(CustomError::TooBigOffset.into());
    }

    let list = db
        .read()
        .await
        .find(FindOptions {
            author: body.author.as_ref().map(|x| x as _),
            content: body.content.as_ref().map(|x| x as _),
            offset: body.offset,
            limit: body.limit,
        })
        .await
        .context("failed to find")
        .map_err(CustomError::Internal)?;

    Ok(list)
}
