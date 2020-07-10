use crate::db::MeigenDatabase;
use crate::db::RegisteredMeigen;

use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone)]
pub struct ApiInstance<TDatabase: MeigenDatabase> {
    db: Arc<RwLock<TDatabase>>,
}

use warp::Filter;

impl<TDatabase: MeigenDatabase> ApiInstance<TDatabase> {
    async fn get_all_entries(db: &Arc<RwLock<TDatabase>>) -> Vec<RegisteredMeigen> {
        db.read().unwrap().meigens().await.unwrap()
    }

    async fn start(self) {
        let all = {
            let db = Arc::clone(&self.db);
            warp::path!("all").map(move || {
                let meigens = &futures::executor::block_on(Self::get_all_entries(&db));
                warp::reply::json(&meigens)
            })
        };

        warp::serve(all).run(([127, 0, 0, 1], 8000)).await;
    }
}

pub async fn launch(db: Arc<RwLock<impl MeigenDatabase>>) {
    let instance = ApiInstance { db };
    instance.start().await;
}
