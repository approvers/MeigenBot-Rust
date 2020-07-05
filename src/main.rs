#![allow(dead_code)]
#![deny(clippy::all)]

use serenity::client::Client;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::{Context, EventHandler};
use std::env;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

mod command_registry;
mod commands;
mod db;
mod make_error_enum;
mod message_parser;

use db::filedb::FileDB;

const CONF_FILE_NAME: &str = "./conf.yaml";
const NEW_CONF_FILE_NAME: &str = "./conf.new.yaml";

const KAWAEMON_ID: u64 = 391857452360007680;

const MESSAGE_MAX_LENGTH: usize = 1000;

enum ClientEvent {
    OnReady(Context),
    OnMessage(Box<Message>),
}

struct BotEvHandler {
    channel: Mutex<mpsc::Sender<ClientEvent>>,
}

#[async_trait]
impl EventHandler for BotEvHandler {
    async fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let event = ClientEvent::OnReady(ctx);

        self.channel.lock().unwrap().send(event).unwrap();
    }

    async fn message(&self, _: Context, new_message: Message) {
        let event = ClientEvent::OnMessage(Box::new(new_message));

        self.channel.lock().unwrap().send(event).unwrap();
    }
}

// #[tokio::main]を使わないのは、なんかこうruntimeを自分で作りたいからです。
async fn async_main() {
    let log_level = {
        if cfg!(debug) {
            4 //trace
        } else {
            2 //info
        }
    };

    stderrlog::new()
        .module(module_path!())
        .verbosity(log_level)
        .timestamp(stderrlog::Timestamp::Second)
        .init()
        .unwrap();

    if env::args().any(|x| x == "--newconf") {
        FileDB::new(NEW_CONF_FILE_NAME).save().await.unwrap();
        return;
    }

    let token = env::var("DISCORD_TOKEN").expect("Set DISCORD_TOKEN");

    let mut db = FileDB::load(CONF_FILE_NAME)
        .await
        .expect("Open database file failed");

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
                println!("Bot is ready!");
                context = Some(ctx);
            }

            ClientEvent::OnMessage(msg) => {
                let ctx = context.as_ref().expect("event was called before ready");

                let is_admin = msg.author.id == KAWAEMON_ID;

                if let Some(parsed_msg) = message_parser::parse_message(&msg) {
                    let send_msg = {
                        match command_registry::call_command(&mut db, parsed_msg, is_admin).await {
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

fn main() {
    let mut runtime = tokio::runtime::Runtime::new().expect("Initializing tokio failed");
    runtime.block_on(async_main());
}

async fn send_message(text: &impl std::fmt::Display, channel_id: ChannelId, http: &Arc<Http>) {
    if let Err(e) = channel_id.say(http, text).await {
        println!("Failed to send message \"{}\"\n{}", &text, e);
    }
}
