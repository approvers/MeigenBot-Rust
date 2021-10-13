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
        #[cfg(feature = "api_graphql")]
        let graphql_filter = {
            let ctx = super::graphql::Context {
                db: Arc::clone(&self.db),
                auth: self.auth.clone(),
            };

            let ctx = inject(ctx).map_err(warp::filter::Internal, |e| -> Rejection { match e {} });

            warp::path!("v1" / "graphql").and(
                juniper_warp::make_graphql_filter(super::graphql::schema(), ctx)
                    .map(|_| ())
                    .untuple_one(),
            )
        };

        #[cfg(not(feature = "api_graphql"))]
        let graphql_filter = warp::any();

        let route = graphql_filter
            .and(get(&self.auth, &self.db))
            .or(random(&self.auth, &self.db))
            .or(search(&self.auth, &self.db))
            .recover(recover)
            .with(warp::trace::request());

        fn hoge<T>(_: &T) {
            println!("{}", std::any::type_name::<T>());
        }
        hoge(&route);

        let ip = ip.into();
        tracing::info!("starting server at {}", ip);

        warp::serve(route).bind(ip).await;
    }
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
            let r = super::get(id, db).await.map_err(Rejection::from)?;

            match r {
                Some(m) => Ok(warp::reply::json(&m)),
                None => Err(warp::reject::not_found()),
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
    let (code, msg) = match r.find::<CustomError>() {
        None => return Err(r),
        Some(&CustomError::Internal(ref e)) => {
            tracing::error!("internal error: {:#?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }

        Some(&CustomError::FetchLimitExceeded) => {
            (StatusCode::BAD_REQUEST, "attempted to get too many meigens")
        }

        Some(&CustomError::SearchWordLengthLimitExceeded) => {
            (StatusCode::BAD_REQUEST, "search keyword is too long")
        }

        Some(&CustomError::TooBigOffset) => (StatusCode::BAD_REQUEST, "offset is too big"),
        Some(&CustomError::Authentication) => (StatusCode::UNAUTHORIZED, "unauthorized"),
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({ "error": msg })),
        code,
    ))
}
