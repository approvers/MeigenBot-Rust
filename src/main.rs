#![allow(dead_code)]

mod command;
mod db;
mod entrypoint;
mod model;

use {
    crate::{
        db::mongo::MongoMeigenDatabase, entrypoint::discord_webhook::DiscordWebhookServerOptions,
    },
    std::sync::Arc,
    tokio::sync::RwLock,
};

type Synced<T> = Arc<RwLock<T>>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    DiscordWebhookServerOptions {
        token: std::env::var("DISCORD_TOKEN").unwrap(),
        app_public_key: std::env::var("DISCORD_APP_PUBLIC_KEY").unwrap(),
        db: MongoMeigenDatabase::new(&std::env::var("MONGODB_URI").unwrap())
            .await
            .unwrap(),
    }
    .into_server()
    .unwrap()
    .start(([127, 0, 0, 1], 8080))
    .await
    .unwrap();
}
