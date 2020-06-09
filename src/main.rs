#![allow(non_upper_case_globals)]
use lazy_static::lazy_static;

use serenity::client::Client;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::{Context, EventHandler};
use std::sync::Mutex;

mod botconfig;
mod message_checker;

use botconfig::{BotConfig, MeigenEntry};
use message_checker::trim_empty;

lazy_static! {
    static ref conf: Mutex<BotConfig> = Mutex::new(BotConfig::load());
}

fn main() {
    let token = conf.lock().unwrap().discord_token.clone();
    let handler = BotEvHandler;

    let mut client = Client::new(token, handler).unwrap();
    client.start().unwrap();
}

struct BotEvHandler;

impl EventHandler for BotEvHandler {
    fn ready(&self, _ctx: Context, _data_about_bot: Ready) {
        println!("Bot is ready.");
    }

    fn message(&self, ctx: Context, new_message: Message) {
        if new_message.author.bot {
            return;
        }

        let content = new_message.content.trim();

        if content.is_empty() {
            return;
        }

        let splits = new_message.content.split(" ").collect::<Vec<&str>>();

        if splits.len() <= 2 {
            return;
        }

        if *splits.get(0).unwrap() != "g!meigen" {
            return;
        }

        let author = {
            let temp = (*splits.get(1).unwrap()).trim().to_string();
            trim_empty(&temp)
        };

        let content = {
            let temp = splits
                .iter()
                .skip(2)
                .fold(String::new(), |a, b| format!("{} {}", a, b))
                .trim()
                .to_string();
            trim_empty(&temp)
        };

        if author.is_empty() || content.is_empty() {
            return;
        }

        if author == "id" || author == "print" || author == "del" || author == "random" {
            return;
        }

        if author.len() + content.len() > 300 {
            new_message
                .channel_id
                .say(&ctx.http, "いくらなんでも合計300文字以上は長過ぎません？")
                .unwrap();

            return;
        }

        let entry = MeigenEntry { author, content };
        println!("Meigen: {:?}", &entry);

        let result = conf.lock().unwrap().new_meigen(entry);
        if let Err(e) = result {
            println!("{}", e);
        }
    }
}
