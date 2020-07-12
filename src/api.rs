use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;
use futures::executor::block_on;
use log::info;
use percent_encoding::percent_decode;
use serde::Serialize;
use std::borrow::Cow;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use warp::http::StatusCode;
use warp::path;
use warp::reply;
use warp::reply::json;
use warp::reply::Json;
use warp::Filter;
use warp::Rejection;
use warp::Reply;

mod inner {
    use super::*;

    type Database<D> = Arc<RwLock<D>>;

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

        fn handle_all(db: &Database<D>) -> Json {
            with_report(|| {
                let result = json(&block_on(Self::get_all_entries(&db)));

                (Cow::from("GET /all"), result)
            })
        }

        fn handle_author(db: &Database<D>, filter: String) -> Json {
            with_report(|| {
                let filter = percent_decode(filter.as_bytes()).decode_utf8().unwrap();
                let result = json(&block_on(Self::get_by_author(&db, filter.as_ref())));

                (Cow::from(format!("GET /author/{}", filter)), result)
            })
        }

        async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
            #[derive(Serialize)]
            struct ErrorMessage {
                code: u16,
                message: &'static str,
            }

            let (code, message) = {
                if err.is_not_found() {
                    (StatusCode::NOT_FOUND, "Not found")
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
                }
            };

            let json = json(&ErrorMessage {
                code: code.as_u16(),
                message,
            });

            Ok(reply::with_status(json, code))
        }

        async fn get_all_entries(db: &Database<D>) -> Vec<RegisteredMeigen> {
            db.read().unwrap().meigens().await.unwrap()
        }

        async fn get_by_author(db: &Database<D>, filter: &str) -> Vec<RegisteredMeigen> {
            db.read()
                .unwrap()
                .meigens()
                .await
                .unwrap()
                .drain(..)
                .filter(|x| x.author.contains(&filter))
                .collect()
        }
    }

    #[inline]
    fn with_report<F, R>(f: F) -> R
    where
        F: FnOnce() -> (Cow<'static, str>, R),
    {
        let begin = Instant::now();
        let result = f();
        let took_time = (Instant::now() - begin).as_millis();

        info!("{}: took {}ms", result.0, took_time);
        result.1
    }
}

pub async fn launch(address: impl Into<SocketAddr>, db: Arc<RwLock<impl MeigenDatabase>>) {
    let instance = inner::ApiServer::new(address.into(), db);

    instance.start().await;
}
