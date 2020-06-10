#![allow(dead_code)]

use serenity::client::Client;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::{Context, EventHandler};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

mod botconfig;
mod message_checker;
mod message_solver;

use botconfig::BotConfig;
use message_solver::MessageSolver;

fn main() {
    let conf = BotConfig::load();
    let token = conf.discord_token.clone();

    let (tx, rx) = mpsc::channel();
    let handler = BotEvHandler {
        channel: Mutex::new(tx),
    };

    thread::spawn(move || {
        let mut client = Client::new(token, handler).unwrap();
        client.start().unwrap();
    });

    let mut solver = MessageSolver::new(conf);
    let mut context = None;
    for event in rx {
        match event {
            ClientEvent::OnReady(ctx) => {
                println!("Bot is ready!");
                context = Some(ctx);
            }
            ClientEvent::OnMessage(msg) => {
                let ctx = context.as_ref().expect("event was called before ready");

                match solver.solve(&msg) {
                    Ok(e) => {
                        if let Some(text) = e {
                            send_message(&text, msg.channel_id, &ctx.http)
                        }
                    }
                    Err(e) => send_message(&e, msg.channel_id, &ctx.http),
                }
            }
        }
    }
}

fn send_message(text: &impl std::fmt::Display, channel_id: ChannelId, http: &Arc<Http>) {
    if let Err(e) = channel_id.say(http, text) {
        println!("Failed to send message \"{}\"\n{}", &text, e);
    }
}

enum ClientEvent {
    OnReady(Context),
    OnMessage(Message),
}

struct BotEvHandler {
    channel: Mutex<mpsc::Sender<ClientEvent>>,
}

impl EventHandler for BotEvHandler {
    fn ready(&self, ctx: Context, _data_about_bot: Ready) {
        let event = ClientEvent::OnReady(ctx);

        self.channel.lock().unwrap().send(event).unwrap();
    }

    fn message(&self, _: Context, new_message: Message) {
        let event = ClientEvent::OnMessage(new_message);

        self.channel.lock().unwrap().send(event).unwrap();
    }
}
