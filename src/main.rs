#![allow(dead_code)]
#![deny(clippy::all)]

use async_trait::async_trait;
use log::info;
use serenity::client::Client;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::{Context, EventHandler};
use std::env;
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};

mod api;
mod cli;
mod command_registry;
mod commands;
mod db;
mod make_error_enum;
mod message_parser;

use db::filedb::FileDB;
use db::mongodb::MongoDB;
use db::MeigenDatabase;

const ADMIN_ID: &[u64] = &[
    391857452360007680, //kawaemon
];

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
    simple_logger::init_with_level(log::Level::Info).unwrap();

    let mut runtime = tokio::runtime::Builder::new()
        .enable_time()
        .enable_io()
        .threaded_scheduler()
        .build()
        .expect("Failed to build tokio runtime.");

    runtime.block_on(async_main());
}

async fn async_main() {
    let options = match cli::parse() {
        Some(t) => t,
        None => return,
    };

    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN");
    let port = env::var("PORT")
        .expect("Set PORT for api server")
        .parse()
        .expect("PORT variable is not collect value. expected u16.");

    use cli::Database;
    match options.database {
        Database::File => {
            let db = FileDB::load(&options.dest)
                .await
                .expect("Open database file failed");
            main_routine(token, port, db).await
        }

        Database::Mongo => {
            let db = MongoDB::new(&options.dest)
                .await
                .expect("Connect to mongo db failed");
            main_routine(token, port, db).await
        }
    };
}

async fn main_routine(token: String, port: u16, db: impl MeigenDatabase) {
    let db = Arc::new(RwLock::new(db));

    info!("Starting Api server at 127.0.0.1:{}", port);
    tokio::spawn(api::launch(([127, 0, 0, 1], port), Arc::clone(&db)));

    let (tx, rx) = mpsc::channel();
    let handler = BotEvHandler {
        channel: Mutex::new(tx),
    };

    tokio::spawn(async {
        Client::new(token)
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
                println!("Discord Bot is ready!");
                context = Some(ctx);
            }

            ClientEvent::OnMessage(msg) => {
                let ctx = context.as_ref().expect("event was called before ready");

                let is_admin = ADMIN_ID.contains(&msg.author.id.0);

                if let Some(parsed_msg) = message_parser::parse_message(&msg) {
                    let send_msg = {
                        match command_registry::call_command(&db, parsed_msg, is_admin).await {
                            Ok(m) => m,
                            Err(e) => e.to_string(),
                        }
                    };

                    send_message(&send_msg, msg.channel_id, &ctx.http).await;
                }
            }
        }
    }
}

async fn send_message(text: &impl std::fmt::Display, channel_id: ChannelId, http: &Arc<Http>) {
    if let Err(e) = channel_id.say(http, text).await {
        println!("Failed to send message \"{}\"\n{}", &text, e);
    }
}
