mod command;
mod db;
mod entrypoint;
mod model;
mod util;

use {
    crate::{
        db::{mem::MemoryMeigenDatabase, mongo::MongoMeigenDatabase, MeigenDatabase},
        entrypoint::{console::Console, discord_webhook::DiscordWebhookServerOptions},
    },
    anyhow::{bail, Context, Result},
    std::sync::Arc,
    tokio::sync::RwLock,
};

type Synced<T> = Arc<RwLock<T>>;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    tokio::runtime::Builder::new()
        .threaded_scheduler()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?
        .block_on(async_main())
}

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("failed to get {} environment variable", name))
}

async fn async_main() -> Result<()> {
    let db_mode = env_var("DB_MODE")?;

    match db_mode.as_str() {
        "mongo" => {
            run(MongoMeigenDatabase::new(&env_var("MONGODB_URI")?)
                .await
                .context("failed to get mongodb instance")?)
            .await
        }
        "mem" => run(MemoryMeigenDatabase::new()).await,
        _ => bail!("DB_MODE environment variable must be either \"mongo\" or \"mem\""),
    }?;

    async fn run(db: impl MeigenDatabase) -> Result<()> {
        match env_var("MODE")?.as_str() {
            "discord_webhook" => run_discord_webhook_server(db).await,
            "console" => run_on_console(db).await,
            _ => {
                bail!("MODE environment variable must be either \"discord_webhook\" or \"console\"")
            }
        };

        Ok(())
    }

    Ok(())
}

async fn run_on_console(db: impl MeigenDatabase) {
    Console::new(db).run().await;
}

async fn run_discord_webhook_server(db: impl MeigenDatabase) {
    let port = std::env::var("PORT")
        .as_ref()
        .map(|x| x.as_str())
        .unwrap_or("8080")
        .parse()
        .unwrap();

    DiscordWebhookServerOptions {
        app_public_key: std::env::var("DISCORD_APP_PUBLIC_KEY").unwrap(),
        db,
    }
    .into_server()
    .unwrap()
    .start(([0, 0, 0, 0], port))
    .await
    .unwrap();
}
