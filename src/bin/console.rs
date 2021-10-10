use anyhow::{Context, Result};
#[cfg(feature = "memorydb")]
use meigen_bot_rust::db::mem::MemoryMeigenDatabase;
#[cfg(feature = "mongodb_")]
use meigen_bot_rust::db::mongo::MongoMeigenDatabase;
use meigen_bot_rust::entrypoint::console::Console;

#[cfg(all(not(feature = "memorydb"), not(feature = "mongodb_")))]
compile_error!("memorydb OR mongodb must be enabled, not both.");
#[cfg(all(feature = "memorydb", feature = "mongodb_"))]
compile_error!("memorydb OR mongodb must be enabled, not both.");

fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let use_ansi = env_var("NO_COLOR").is_err();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_ansi(use_ansi)
        .init();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?
        .block_on(async_main())
}

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("failed to get {} environment variable", name))
}

async fn async_main() -> Result<()> {
    #[cfg(feature = "memorydb")]
    let db = MemoryMeigenDatabase::new();

    #[cfg(feature = "mongodb_")]
    let db = MongoMeigenDatabase::new(&env_var("MONGODB_URI")?)
        .await
        .context("failed to get mongodb instance")?;

    Console::new(db).run().await;

    Ok(())
}
