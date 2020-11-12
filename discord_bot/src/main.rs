#![allow(dead_code)]
#![deny(clippy::all)]

use serenity::async_trait;
use serenity::client::Client;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};
use std::env;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

mod command_registry;
mod commands;
mod message_parser;

use command_registry::call_command;
use db::{filedb::FileDB, mongodb::MongoDB, MeigenDatabase};
use message_parser::parse_message;

#[macro_export]
macro_rules! make_error_enum {
    ($enum_name:ident; $($variant:ident $func_name:ident($($($vars:ident),+ $(,)?)?) => $format:expr),+ $(,)?) => {
        #[derive(Debug)]
        pub enum $enum_name {
            $($variant(String),)+
        }

        impl $enum_name {
            $ (
                pub fn $func_name($($($vars: impl std::fmt::Display,)*)?) -> $enum_name {
                    $enum_name::$variant(format!($format, $($($vars),+)?))
                }
            )+
        }

        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $($enum_name::$variant(text) => write!(f, "{}", text),)+
                }
            }
        }
    };
}

struct BotEvHandler<D: MeigenDatabase> {
    db: Arc<RwLock<D>>,
    admin_id: Vec<u64>,
}

#[async_trait]
impl<D> EventHandler for BotEvHandler<D>
where
    D: MeigenDatabase,
{
    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(parsed_msg) = parse_message(&msg.content) {
            let is_admin = self.admin_id.iter().any(|x| *x == msg.author.id.0);

            let begin = Instant::now();
            let cmd_result = call_command(&self.db, parsed_msg, is_admin).await;
            log::info!("\"{}\" took {}ms", msg.content, begin.elapsed().as_millis());

            let message = match cmd_result {
                Ok(r) => r,
                Err(e) => e.to_string(),
            };

            if let Err(e) = msg.channel_id.say(&ctx.http, &message).await {
                log::error!("Failed to send message \"{}\"\n{}", &message, e);
            }
        }
    }

    async fn ready(&self, _: Context, _: Ready) {
        log::info!("Discord bot is ready!");
    }
}

fn main() {
    pretty_env_logger::init();

    tokio::runtime::Builder::new()
        .enable_time()
        .enable_io()
        .basic_scheduler()
        .build()
        .expect("Failed to build tokio runtime.")
        .block_on(async_main());
}

async fn async_main() {
    let db = env::var("DB_TYPE").expect("Set DB_TYPE to either MONGO or FILE");
    let dest = env::var("DB_DEST").expect("Set DB_DESt");
    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN");
    let admin_id = env::var("ADMIN_DISCORD_ID")
        .expect("Set admin discord id")
        .parse()
        .expect("Invalid admin discord id.");
    let admin_ids = vec![admin_id];

    match db.as_str() {
        "FILE" => {
            let db = FileDB::load(&dest)
                .await
                .expect("Failed to open database file");
            start(token, db, admin_ids).await
        }

        "MONGO" => {
            let db = MongoDB::new(&dest)
                .await
                .expect("Failed to connect to mongo db");
            start(token, db, admin_ids).await
        }

        _ => panic!("Set DB_TYPE to either MONGO or FILE"),
    };
}

async fn start(token: String, db: impl MeigenDatabase, admin_id: Vec<u64>) {
    let db = Arc::new(RwLock::new(db));

    Client::builder(token)
        .event_handler(BotEvHandler { db, admin_id })
        .await
        .expect("Initializing serenity failed.")
        .start()
        .await
        .expect("Serenity returns unknown error.");
}
