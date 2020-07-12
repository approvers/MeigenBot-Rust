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
use warp::reply::WithStatus;
use warp::Filter;
use warp::Rejection;

mod inner {
    use super::*;

    type Database<D> = Arc<RwLock<D>>;

    #[derive(Serialize)]
    struct ErrorMessage {
        code: u16,
        message: Cow<'static, str>,
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
                    let log_msg = Cow::from("GET /all");
                    let json = json(&a);
                    let reply = reply::with_status(json, StatusCode::OK);

                    (log_msg, reply)
                }

                Err(e) => {
                    let log_msg = Cow::from(format!("GET /all Error: {}", e));
                    let error = ErrorMessage {
                        code: 500,
                        message: Cow::from("Internal error"),
                    };

                    let json = json(&error);
                    let reply = reply::with_status(json, StatusCode::INTERNAL_SERVER_ERROR);
                    (log_msg, reply)
                }
            })
        }

        fn handle_author(db: &Database<D>, filter: String) -> WithStatus<Json> {
            with_report(|| {
                let filter = percent_decode(filter.as_bytes()).decode_utf8().unwrap();

                match &block_on(Self::get_by_author(&db, filter.as_ref())) {
                    Ok(a) => {
                        let log_msg = Cow::from(format!("GET /author/{}", filter));
                        let json = json(&a);
                        let reply = reply::with_status(json, StatusCode::OK);

                        (log_msg, reply)
                    }

                    Err(e) => {
                        let log_msg = Cow::from(format!("GET /author/{} Error: {}", filter, e));
                        let error = ErrorMessage {
                            code: 500,
                            message: Cow::from("Internal error"),
                        };

                        let json = json(&error);
                        let reply = reply::with_status(json, StatusCode::INTERNAL_SERVER_ERROR);

                        (log_msg, reply)
                    }
                }
            })
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
                message: Cow::from(message),
            });

            Ok(reply::with_status(json, code))
        }

        async fn get_all_entries(db: &Database<D>) -> Result<Vec<RegisteredMeigen>, D::Error> {
            db.read().unwrap().meigens().await
        }

        async fn get_by_author(
            db: &Database<D>,
            filter: &str,
        ) -> Result<Vec<RegisteredMeigen>, D::Error> {
            let r = db
                .read()
                .unwrap()
                .meigens()
                .await?
                .drain(..)
                .filter(|x| x.author.contains(&filter))
                .collect();

            Ok(r)
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
