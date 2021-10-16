use anyhow::{Context, Result};
#[cfg(feature = "memorydb")]
use meigen_bot_rust::db::mem::MemoryMeigenDatabase;
#[cfg(feature = "mongodb_")]
use meigen_bot_rust::db::mongo::MongoMeigenDatabase;
#[cfg(feature = "api_auth_always_pass")]
use meigen_bot_rust::entrypoint::api::auth::AlwaysPass;
#[cfg(not(feature = "api_auth_always_pass"))]
use meigen_bot_rust::entrypoint::api::auth::GAuth;
use meigen_bot_rust::entrypoint::api::warp::HttpApiServer;

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

    let port = std::env::var("PORT")
        .as_ref()
        .map(|x| x.as_str())
        .unwrap_or("8080")
        .parse()
        .unwrap();

    #[cfg(not(feature = "api_auth_always_pass"))]
    let authenticator = {
        let gauth_endpoint = env_var("GAUTH_ENDPOINT")?;
        let gauth_endpoint: &'static str = Box::leak(gauth_endpoint.into_boxed_str());
        GAuth::new(gauth_endpoint)
    };

    #[cfg(feature = "api_auth_always_pass")]
    let authenticator = AlwaysPass;

    HttpApiServer::new(db, authenticator)
        .start(([0, 0, 0, 0], port))
        .await;

    Ok(())
}
