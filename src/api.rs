use crate::db::MeigenDatabase;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

mod api;

pub async fn launch(address: impl Into<SocketAddr>, db: Arc<RwLock<impl MeigenDatabase>>) {
    let instance = api::ApiServer::new(address.into(), db);

    instance.start().await;
}
