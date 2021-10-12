use std::{convert::Infallible, net::SocketAddr, sync::Arc, time::Duration};

use anyhow::{Context as _, Result};
use reqwest::StatusCode;
use serde::{de::DeserializeOwned, Deserialize};
use tokio::sync::RwLock;
use tokio_stream::StreamExt;
use warp::{Filter, Rejection};

use crate::{
    db::{FindOptions, MeigenDatabase},
    Synced,
};

const SEARCH_STRING_LENGTH_LIMIT: usize = 100;
const MAX_FETCH_COUNT: usize = 50;

pub struct HttpApi<D: MeigenDatabase> {
    db: Synced<D>,
    gauth_endpoint: String,
    client: reqwest::Client,
}

impl<D: MeigenDatabase> HttpApi<D> {
    pub fn new(db: D, gauth_endpoint: String) -> Self {
        Self {
            db: Arc::new(RwLock::new(db)),
            gauth_endpoint,
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        }
    }

    pub async fn start(self, ip: impl Into<SocketAddr>) {
        // gauth endpoint should be so small, and this function is not intended to be called many
        // times, so this should not be big problem.
        let gauth_endpoint: &'static str = Box::leak(self.gauth_endpoint.into_boxed_str());
        let client = self.client;

        let route = warp::body::content_length_limit(3 * 1024)
            .and(
                warp::path!("v1" / u32)
                    .and(warp::post())
                    .and(auth_filter(client.clone(), gauth_endpoint))
                    .and(inject(Arc::clone(&self.db)))
                    .and_then(get)
                    .or(warp::path!("v1" / "random")
                        .and(warp::post())
                        .and(auth_filter(client.clone(), gauth_endpoint))
                        .and(inject(Arc::clone(&self.db)))
                        .and_then(random))
                    .or(warp::path!("v1" / "search")
                        .and(warp::post())
                        .and(auth_filter(client.clone(), gauth_endpoint))
                        .and(inject(Arc::clone(&self.db)))
                        .and_then(search)),
            )
            .recover(recover)
            .with(warp::trace::request());

        warp::serve(route).bind(ip.into()).await;
    }
}

fn auth_filter<'a, T: 'a>(
    client: reqwest::Client,
    endpoint: &'a str,
) -> impl Filter<Extract = (T,), Error = Rejection> + Clone + 'a
where
    T: AsRef<AuthToken> + DeserializeOwned + Send,
{
    warp::body::json::<T>()
        .and(inject(client))
        .and(inject(endpoint))
        .and_then(auth)
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

fn internal_error(e: anyhow::Error) -> Rejection {
    CustomError::Internal(e).into()
}

#[derive(Deserialize)]
struct AuthToken {
    token: String,
}

impl AsRef<AuthToken> for AuthToken {
    fn as_ref(&self) -> &AuthToken {
        &self
    }
}
macro_rules! as_auth_token {
    ($name: ident) => {
        impl AsRef<AuthToken> for $name {
            fn as_ref(&self) -> &AuthToken {
                &self.auth
            }
        }
    };
}

async fn auth<T>(request_body: T, client: reqwest::Client, endpoint: &str) -> Result<T, Rejection>
where
    T: AsRef<AuthToken>,
{
    let token = &request_body.as_ref().token;

    let result = client
        .post(endpoint)
        .body(serde_json::json!({ "token": token }).to_string())
        .send()
        .await
        .context("failed to send auth request")
        .map_err(internal_error)?;

    let status = result.status();
    let body = result
        .text()
        .await
        .context("failed to receive body")
        .map_err(internal_error)?;

    #[derive(Deserialize)]
    struct Response {
        #[allow(dead_code)]
        user_id: String,
    }

    match status {
        StatusCode::OK => {
            let _response = serde_json::from_str(&body)
                .context("failed to deserialize response json")
                .map_err(internal_error)?;

            Ok(request_body)
        }

        StatusCode::UNAUTHORIZED => Err(CustomError::Authentication.into()),

        p => Err(internal_error(anyhow::anyhow!(
            "auth request failed: status: {}, body: {}",
            p,
            body
        ))),
    }
}

async fn get(
    id: u32,
    _: AuthToken,
    db: Synced<impl MeigenDatabase>,
) -> Result<impl warp::Reply, Rejection> {
    let entry = db.read().await.load(id).await.map_err(internal_error)?;

    if let Some(m) = entry {
        Ok(warp::reply::with_status(
            warp::reply::json(&m),
            StatusCode::OK,
        ))
    } else {
        Err(CustomError::NotFound.into())
    }
}

#[derive(Deserialize)]
struct RandomRequest {
    #[serde(flatten)]
    auth: AuthToken,
    count: usize,
}

as_auth_token!(RandomRequest);

async fn random(
    body: RandomRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<impl warp::Reply, Rejection> {
    if body.count > MAX_FETCH_COUNT {
        return Err(CustomError::FetchLimitExceeded.into());
    }

    let max = db
        .read()
        .await
        .get_current_id()
        .await
        .context("failed to get current id")
        .map_err(internal_error)?;

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
    .map_err(internal_error)?;

    list.sort_unstable_by_key(|x| x.id);

    Ok(warp::reply::json(&list))
}

#[derive(Deserialize)]
struct FindRequest {
    #[serde(flatten)]
    auth: AuthToken,

    offset: u32,
    limit: u8,
    author: Option<String>,
    content: Option<String>,
}

as_auth_token!(FindRequest);

async fn search(
    body: FindRequest,
    db: Synced<impl MeigenDatabase>,
) -> Result<impl warp::Reply, Rejection> {
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
        .map_err(internal_error)?;

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
        .map_err(internal_error)?;

    Ok(warp::reply::json(&list))
}
