use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;
use futures::executor::block_on;
use log::info;
use percent_encoding::percent_decode;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use warp::path;
use warp::reply::json;
use warp::reply::Json;
use warp::Filter;

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

            let routes = warp::get().and(all.or(by_author));
            warp::serve(routes).run(self.address).await;
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

        fn handle_all(db: &Database<D>) -> Json {
            with_report(|| {
                let result = json(&block_on(Self::get_all_entries(&db)));

                (format!("GET /all"), result)
            })
        }

        fn handle_author(db: &Database<D>, filter: String) -> Json {
            with_report(|| {
                let filter = percent_decode(filter.as_bytes()).decode_utf8().unwrap();
                let result = json(&block_on(Self::get_by_author(&db, filter.as_ref())));

                (format!("GET /author/{}", filter), result)
            })
        }
    }

    #[inline]
    fn with_report<F, R>(f: F) -> R
    where
        F: FnOnce() -> (String, R),
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
