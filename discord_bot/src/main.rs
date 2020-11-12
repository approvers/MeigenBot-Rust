#![allow(dead_code)]
#![deny(clippy::all)]

use log::{error, info};
use serenity::async_trait;
use serenity::client::Client;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};
use std::env;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

mod cli;
mod command_registry;
mod commands;
mod message_parser;
mod report;

use cli::Database;
use command_registry::call_command;
use db::filedb::FileDB;
use db::mongodb::MongoDB;
use db::MeigenDatabase;
use message_parser::parse_message;
use report::with_time_report_async;

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

enum ClientEvent {
    OnReady(Context),
    OnMessage(Box<Message>),
}

struct BotEvHandler {
    channel: Mutex<mpsc::Sender<ClientEvent>>,
}

#[async_trait]
impl EventHandler for BotEvHandler {
    async fn message(&self, _: Context, new_message: Message) {
        let event = ClientEvent::OnMessage(Box::new(new_message));

        self.channel.lock().unwrap().send(event).unwrap();
    }

    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let event = ClientEvent::OnReady(ctx);

        self.channel.lock().unwrap().send(event).unwrap();
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
    let options = match cli::parse() {
        Some(t) => t,
        None => return,
    };

    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN");
    let admin_id = env::var("ADMIN_DISCORD_ID")
        .expect("Set admin discord id")
        .parse()
        .expect("Invalid admin discord id.");
    let admin_ids = &[admin_id];

    match options.database {
        Database::File => {
            let db = FileDB::load(&options.dest)
                .await
                .expect("Open database file failed");
            start(token, db, admin_ids).await
        }

        Database::Mongo => {
            let db = MongoDB::new(&options.dest)
                .await
                .expect("Connect to mongo db failed");
            start(token, db, admin_ids).await
        }
    };
}

async fn start(token: String, db: impl MeigenDatabase, admin_id: &[u64]) {
    let db = Arc::new(RwLock::new(db));

    let (tx, rx) = mpsc::channel();
    let handler = BotEvHandler {
        channel: Mutex::new(tx),
    };

    tokio::spawn(async {
        Client::builder(token)
            .event_handler(handler)
            .await
            .expect("Initializing serenity failed.")
            .start()
            .await
            .expect("Serenity returns unknown error.");
    });

    let mut context = None;
    for event in rx {
        match event {
            ClientEvent::OnReady(ctx) => {
                info!("Discord Bot is ready!");
                context = Some(ctx);
            }

            ClientEvent::OnMessage(msg) => {
                if let Some(parsed_msg) = parse_message(&msg) {
                    let ctx = context.as_ref().expect("event was called before ready");
                    let is_admin = admin_id.iter().any(|x| *x == msg.author.id.0);

                    let cmd_result = with_time_report_async(
                        call_command(&db, parsed_msg, is_admin),
                        |r| match r.as_ref() {
                            Ok(_) => format!("\"{}\" was ok", &msg.content),
                            Err(e) => format!("\"{}\" was not ok: {:?}", &msg.content, &e),
                        },
                    )
                    .await;

                    let message = match cmd_result {
                        Ok(r) => r,
                        Err(e) => e.to_string(),
                    };

                    if let Err(e) = msg.channel_id.say(&ctx.http, &message).await {
                        error!("Failed to send message \"{}\"\n{}", &message, e);
                    }
                }
            }
        }
    }
}
