use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use anyhow::Result;
use reqwest::StatusCode;
use tokio::sync::RwLock;
use warp::{filter::FilterBase, Filter, Rejection, Reply};

use super::{auth::Authenticator, CustomError};
use crate::{db::MeigenDatabase, Synced};

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
        let route = graphql(&self.auth, &self.db)
            .or(get(&self.auth, &self.db))
            .or(random(&self.auth, &self.db))
            .or(search(&self.auth, &self.db))
            .recover(recover)
            .with(warp::trace::request());

        let ip = ip.into();
        tracing::info!("starting server at {}", ip);

        warp::serve(route).bind(ip).await;
    }
}

#[cfg(feature = "api_graphql")]
fn graphql(
    auth: &impl Authenticator,
    db: &Synced<impl MeigenDatabase>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let ctx = super::graphql::Context { db: Arc::clone(db) };
    let ctx = inject(ctx).map_err(warp::filter::Internal, |e| -> Rejection { match e {} });

    warp::path!("v1" / "graphql")
        .and(auth_filter(auth.clone()))
        .and(juniper_warp::make_graphql_filter(
            super::graphql::schema(),
            ctx,
        ))
}

#[cfg(not(feature = "api_graphql"))]
fn graphql(
    auth: &impl Authenticator,
    db: &Synced<impl MeigenDatabase>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::any()
}

fn get(
    auth: &impl Authenticator,
    db: &Synced<impl MeigenDatabase>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("v1" / u32)
        .and(warp::get())
        .and(auth_filter(auth.clone()))
        .and(inject(Arc::clone(db)))
        .and_then(|id, db| async move {
            match super::get(id, db).await {
                Ok(Some(m)) => Ok(warp::reply::json(&m)),
                Ok(None) => Err(warp::reject::not_found()),
                Err(e) => Err(Rejection::from(e)),
            }
        })
}

fn random(
    auth: &impl Authenticator,
    db: &Synced<impl MeigenDatabase>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("v1" / "random")
        .and(warp::get())
        .and(auth_filter(auth.clone()))
        .and(warp::query::query())
        .and(inject(Arc::clone(db)))
        .and_then(|query, db| async {
            match super::random(query, db).await {
                Ok(t) => Ok(warp::reply::json(&t)),
                Err(e) => Err(Rejection::from(e)),
            }
        })
}

fn search(
    auth: &impl Authenticator,
    db: &Synced<impl MeigenDatabase>,
) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    warp::path!("v1" / "random")
        .and(warp::get())
        .and(auth_filter(auth.clone()))
        .and(warp::query::query())
        .and(inject(Arc::clone(db)))
        .and_then(|query, db| async {
            match super::search(query, db).await {
                Ok(t) => Ok(warp::reply::json(&t)),
                Err(e) => Err(Rejection::from(e)),
            }
        })
}

fn auth_filter<A: Authenticator>(auth: A) -> impl Filter<Extract = (), Error = Rejection> + Clone {
    warp::header::header::<String>("gauth-token")
        .and(inject(auth))
        .and_then(|token: String, auth: A| async move {
            auth.auth(&token)
                .await
                .map_err(|e| match e {
                    super::auth::Error::Internal(e) => CustomError::Internal(e),
                    super::auth::Error::InvalidToken => CustomError::Authentication,
                })
                .map_err(Rejection::from)
        })
        .map(|_| ())
        .untuple_one()
}

fn inject<T>(t: T) -> impl Filter<Extract = (T,), Error = Infallible> + Send + Clone
where
    T: Send + Clone,
{
    warp::any().map(move || t.clone())
}

impl warp::reject::Reject for CustomError {}

async fn recover(r: Rejection) -> Result<impl warp::Reply, Rejection> {
    if r.is_not_found() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({ "error": "not found" })),
            StatusCode::NOT_FOUND,
        ));
    }

    let ce = match r.find::<CustomError>() {
        Some(t) => t,
        None => return Err(r),
    };

    let (code, msg) = match *ce {
        CustomError::Internal(ref e) => {
            tracing::error!("internal error: {:#?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, ce.describe())
        }

        CustomError::SearchWordLengthLimitExceeded => (StatusCode::BAD_REQUEST, ce.describe()),

        CustomError::FetchLimitExceeded => (StatusCode::BAD_REQUEST, ce.describe()),
        CustomError::TooBigOffset => (StatusCode::BAD_REQUEST, ce.describe()),
        CustomError::Authentication => (StatusCode::UNAUTHORIZED, ce.describe()),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "error": msg })),
        code,
    ))
}
