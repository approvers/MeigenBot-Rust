#![allow(dead_code)]
#![deny(clippy::all)]

use async_trait::async_trait;
use serenity::client::Client;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};
use std::env;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

mod api;
mod cli;
mod command_registry;
mod commands;
mod db;
mod make_error_enum;
mod message_parser;

use cli::Database;
use command_registry::call_command;
use db::filedb::FileDB;
use db::mongodb::MongoDB;
use db::MeigenDatabase;
use message_parser::parse_message;

struct BotEvHandler<D: MeigenDatabase> {
    db: Arc<RwLock<D>>,
    admin_id: Vec<u64>,
}

#[async_trait]
impl<D: MeigenDatabase> EventHandler for BotEvHandler<D> {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Some(parsed_msg) = parse_message(&msg.content) {
            let is_admin = self.admin_id.iter().any(|x| *x == msg.author.id.0);

            let cmd_result = call_command(&self.db, parsed_msg, is_admin).await;

            let message = match cmd_result {
                Ok(r) => {
                    log::info!("\"{}\" was ok", &msg.content);
                    r
                }

                Err(e) => {
                    log::info!("\"{}\" was failed: {:?}", &msg.content, &e);
                    e.to_string()
                }
            };

            if let Err(e) = msg.channel_id.say(&ctx.http, &message).await {
                log::error!("Failed to send message \"{}\"\n{}", &message, e);
            }
        }
    }

    async fn ready(&self, _: Context, _: Ready) {
        log::info!("Discord Bot is ready!");
    }
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

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

    let mut runtime = tokio::runtime::Builder::new()
        .enable_time()
        .enable_io()
        .threaded_scheduler()
        .build()
        .expect("Failed to build tokio runtime.");

    match options.database {
        Database::File => {
            let dbfut = FileDB::load(&options.dest);
            runtime.block_on(async_main(token, port, dbfut, admin_ids));
        }

        Database::Mongo => {
            let dbfut = MongoDB::new(&options.dest);
            runtime.block_on(async_main(token, port, dbfut, admin_ids));
        }
    };
}

async fn async_main<T, D>(token: String, port: u16, dbfut: T, admin_id: &[u64])
where
    D: MeigenDatabase,
    T: Future<Output = Result<D, D::Error>>,
{
    let raw_db = dbfut.await.expect("failed to create database instance");
    let db = Arc::new(RwLock::new(raw_db));

    tokio::spawn(api::launch(([0, 0, 0, 0], port), Arc::clone(&db)));

    let handler = BotEvHandler {
        db,
        admin_id: admin_id.to_vec(),
    };

    Client::new(token)
        .event_handler(handler)
        .await
        .expect("Initializing serenity failed.")
        .start()
        .await
        .expect("Serenity returns unknown error.");
}
