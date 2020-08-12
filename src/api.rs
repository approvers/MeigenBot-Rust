use crate::db::MeigenDatabase;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

mod inner;

pub async fn launch(address: impl Into<SocketAddr>, db: Arc<RwLock<impl MeigenDatabase>>) {
    let instance = inner::ApiServer::new(address.into(), db);

    instance.start().await;
}
