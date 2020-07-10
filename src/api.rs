use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;
use futures::executor::block_on;
use log::info;
use percent_encoding::percent_decode;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use warp::path;
use warp::reply::json;
use warp::reply::Json;
use warp::Filter;

macro_rules! D {
    () => {
        &Arc<RwLock<TDatabase>>
    };
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
    return result.1;
}

impl<TDatabase: MeigenDatabase> ApiInstance<TDatabase> {
    async fn get_all_entries(db: D!()) -> Vec<RegisteredMeigen> {
        db.read().unwrap().meigens().await.unwrap()
    }

    async fn get_by_author(db: D!(), filter: &str) -> Vec<RegisteredMeigen> {
        db.read()
            .unwrap()
            .meigens()
            .await
            .unwrap()
            .drain(..)
            .filter(|x| x.author.contains(&filter))
            .collect()
    }

    fn handle_all(db: D!()) -> Json {
        with_report(|| {
            let result = json(&block_on(Self::get_all_entries(&db)));

            (format!("GET /all"), result)
        })
    }

    fn handle_author(db: D!(), filter: String) -> Json {
        with_report(|| {
            let filter = percent_decode(filter.as_bytes()).decode_utf8().unwrap();
            let result = json(&block_on(Self::get_by_author(&db, filter.as_ref())));

            (format!("GET /author/{}", filter), result)
        })
    }

    async fn start(self) {
        let all = {
            let db = Arc::clone(&self.db);
            path!("all").map(move || Self::handle_all(&db))
        };

        let by_author = {
            let db = Arc::clone(&self.db);
            path!("author" / String).map(move |f| Self::handle_author(&db, f))
        };

        let routes = warp::get().and(all.or(by_author));
        warp::serve(routes).run(([127, 0, 0, 1], 8000)).await;
    }
}
#[derive(Clone)]
pub struct ApiInstance<TDatabase: MeigenDatabase> {
    db: Arc<RwLock<TDatabase>>,
}

pub async fn launch(db: Arc<RwLock<impl MeigenDatabase>>) {
    let instance = ApiInstance { db };
    instance.start().await;
}
