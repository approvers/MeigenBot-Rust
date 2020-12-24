#![allow(dead_code)]
mod db;
mod entrypoint;
mod model;

use entrypoint::discord_webhook::DiscordWebhookServerOptions;
#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    DiscordWebhookServerOptions {
        token: std::env::var("DISCORD_TOKEN").unwrap(),
        app_public_key: std::env::var("APP_PUBLIC_KEY").unwrap(),
    }
    .into_server()
    .unwrap()
    .start(([127, 0, 0, 1], 8080))
    .await
    .unwrap();
}
