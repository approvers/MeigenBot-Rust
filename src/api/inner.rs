use crate::db::{MeigenDatabase, RegisteredMeigen};
use futures::executor::block_on;
use log::info;
use percent_encoding::percent_decode;
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use warp::http::StatusCode;
use warp::reply::{self, json, Json, WithStatus};
use warp::{path, Filter, Rejection};

type Database<D> = Arc<RwLock<D>>;

#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

#[derive(Clone)]
pub struct ApiServer<D: MeigenDatabase> {
    address: SocketAddr,
    db: Database<D>,
}

impl<D: MeigenDatabase> ApiServer<D> {
    pub fn new(address: SocketAddr, db: Database<D>) -> Self {
        Self { address, db }
    }

    pub async fn start(self) {
        let all = {
            let db = Arc::clone(&self.db);
            path!("all").map(move || Self::handle_all(&db))
        };

        let by_author = {
            let db = Arc::clone(&self.db);
            path!("author" / String).map(move |f| Self::handle_author(&db, f))
        };

        let routes = warp::get()
            .and(all.or(by_author))
            .recover(Self::handle_rejection);

        warp::serve(routes).run(self.address).await;
    }

    fn handle_all(db: &Database<D>) -> WithStatus<Json> {
        with_report(|| match block_on(Self::get_all_entries(&db)) {
            Ok(a) => {
                let log_msg = "GET /all";
                let json = json(&a);
                let reply = reply::with_status(json, StatusCode::OK);

                (String::from(log_msg), reply)
            }

            Err(e) => {
                let log_msg = format!("GET /all Error: {}", e);
                (log_msg, Self::internal_error())
            }
        })
    }

    fn handle_author(db: &Database<D>, filter: String) -> WithStatus<Json> {
        with_report(|| {
            let filter = percent_decode(filter.as_bytes()).decode_utf8().unwrap();

            match &block_on(Self::get_by_author(&db, filter.as_ref())) {
                Ok(a) => {
                    let log_msg = format!("GET /author/{}", filter);
                    let json = json(&a);
                    let reply = reply::with_status(json, StatusCode::OK);

                    (log_msg, reply)
                }

                Err(e) => {
                    let log_msg = format!("GET /author/{} Error: {}", filter, e);
                    (log_msg, Self::internal_error())
                }
            }
        })
    }

    fn internal_error() -> WithStatus<Json> {
        let error = ErrorMessage {
            code: 500,
            message: String::from("Internal error"),
        };

        let json = json(&error);
        reply::with_status(json, StatusCode::INTERNAL_SERVER_ERROR)
    }

    async fn handle_rejection(err: Rejection) -> Result<WithStatus<Json>, Infallible> {
        let (code, message) = {
            if err.is_not_found() {
                (StatusCode::NOT_FOUND, "Not found")
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let json = json(&ErrorMessage {
            code: code.as_u16(),
            message: String::from(message),
        });

        Ok(reply::with_status(json, code))
    }

    async fn get_all_entries(db: &Database<D>) -> Result<Vec<RegisteredMeigen>, D::Error> {
        db.read().unwrap().get_all_meigen().await
    }

    async fn get_by_author(
        db: &Database<D>,
        filter: &str,
    ) -> Result<Vec<RegisteredMeigen>, D::Error> {
        db.read().unwrap().search_by_author(filter).await
    }
}

#[inline]
fn with_report<F, R, M>(f: F) -> R
where
    F: FnOnce() -> (M, R),
    M: std::fmt::Display,
{
    let begin = Instant::now();
    let result = f();
    let took_time = (Instant::now() - begin).as_millis();

    info!("{}: took {}ms", result.0, took_time);
    result.1
}
