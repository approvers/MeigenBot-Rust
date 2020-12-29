mod command;
mod db;
mod entrypoint;
mod model;
mod util;

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

    let port = std::env::var("PORT")
        .as_ref()
        .map(|x| x.as_str())
        .unwrap_or("8080")
        .parse()
        .unwrap();

    DiscordWebhookServerOptions {
        app_public_key: std::env::var("DISCORD_APP_PUBLIC_KEY").unwrap(),
        db: MongoMeigenDatabase::new(&std::env::var("MONGODB_URI").unwrap())
            .await
            .unwrap(),
    }
    .into_server()
    .unwrap()
    .start(([0, 0, 0, 0], port))
    .await
    .unwrap();
}
