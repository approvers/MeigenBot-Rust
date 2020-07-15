#![allow(dead_code)]
#![deny(clippy::all)]

use async_trait::async_trait;
use log::{error, info};
use serenity::client::Client;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
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
mod report;

use cli::Database;
use command_registry::call_command;
use db::filedb::FileDB;
use db::mongodb::MongoDB;
use db::MeigenDatabase;
use message_parser::parse_message;
use report::with_time_report_async;

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
            main_routine(token, port, db, admin_ids).await
        }

        Database::Mongo => {
            let db = MongoDB::new(&options.dest)
                .await
                .expect("Connect to mongo db failed");
            main_routine(token, port, db, admin_ids).await
        }
    };
}

async fn main_routine(token: String, port: u16, db: impl MeigenDatabase, admin_id: &[u64]) {
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
                if let Some(parsed_msg) = parse_message(&msg) {
                    let ctx = context.as_ref().expect("event was called before ready");
                    let is_admin = admin_id.iter().any(|x| *x == msg.author.id.0);

                    let cmd_result = with_time_report_async(
                        call_command(&db, parsed_msg, is_admin),
                        |r| match r.as_ref() {
                            Ok(_) => format!("{} was ok", &msg.content),
                            Err(e) => format!("{} was not ok: {:?}", &msg.content, &e),
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
