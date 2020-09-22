#![deny(clippy::all)]

use anyhow::Error;
use anyhow::{anyhow, Context as _, Result};
use async_trait::async_trait;
use serenity::client::Client;
use serenity::http::AttachmentType;
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::model::id::ChannelId;
use serenity::prelude::{Context, EventHandler};
use std::borrow::Cow;
use std::env;
use std::fmt::Display;
use std::path::Path;
use std::sync::Arc;

use meigen::db::filedb::FileDB;
use meigen::db::mongodb::MongoDB;
use meigen::db::MeigenDatabase;
use meigen::MeigenBot;

use interface::{FileEntry, TextBot, TextBotResult, TextMessage};

const KAWAEMON_ID: u64 = 391857452360007680;

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    let token = env::var("DISCORD_TOKEN").context("Set DISCORD_TOKEN")?;

    let mut runtime = tokio::runtime::Builder::new()
        .enable_time()
        .enable_io()
        .threaded_scheduler()
        .build()
        .context("Failed to build tokio runtime.")?;

    match (
        env::var("MEIGEN_DB_FILE_PATH"),
        env::var("MEIGEN_DB_MONGO_URI"),
    ) {
        (Ok(_), Ok(_)) => {
            return Err(anyhow!("Both \"MEIGEN_DB_FILE_PATH\" and \"MEIGEN_DB_MONGO_URI\" are defined. Please unset one of them."));
        }

        (Err(_), Err(_)) => {
            return Err(anyhow!("Both \"MEIGEN_DB_FILE_PATH\" and \"MEIGEN_DB_MONGO_URI\" are not defined. Please set one of them."));
        }

        (Ok(file_path), Err(_)) => {
            log::info!("Using FileDB for Meigen Bot");

            let db = {
                if Path::new(&file_path).exists() {
                    runtime
                        .block_on(FileDB::load(&file_path))
                        .context("Failed to load FileDB")?
                } else {
                    FileDB::new(&file_path)
                }
            };

            runtime.block_on(async_main(token, db));
        }

        (Err(_), Ok(mongo_uri)) => {
            log::info!("Using MongoDB for Meigen Bot");

            let db = runtime
                .block_on(MongoDB::new(&mongo_uri))
                .context("Failed to connect to MongoDB")?;

            runtime.block_on(async_main(token, db));
        }
    };

    Ok(())
}

async fn async_main<D>(token: String, db: D)
where
    D: MeigenDatabase,
{
    let handler = BotEvHandler {
        meigen: MeigenBot::new(db),
    };

    Client::new(token)
        .event_handler(handler)
        .await
        .expect("Failed to initialize serenity")
        .start()
        .await
        .expect("Serenity threw unknown error");
}

struct BotEvHandler<D: MeigenDatabase> {
    meigen: meigen::MeigenBot<D>,
}

#[async_trait]
impl<D: MeigenDatabase> EventHandler for BotEvHandler<D> {
    async fn message(&self, ctx: Context, msg: Message) {
        let text_message = TextMessage {
            content: &msg.content,
            is_kawaemon: msg.author.id == KAWAEMON_ID,
        };

        let result = self.meigen.on_message(text_message).await;

        match result {
            TextBotResult::Ok {
                msg: send_message,
                files,
            } => say(msg.channel_id, &ctx.http, send_message, files).await,

            TextBotResult::ExpectedError(e) => {
                say(msg.channel_id, &ctx.http, Error::new(e), None).await
            }

            TextBotResult::UnexpectedError(e) => {
                let send_message = format!(
                    "HOW THE FUCK <@{}> MADE A BUG\n```{:?}```",
                    KAWAEMON_ID,
                    Error::new(e)
                );

                say(msg.channel_id, &ctx.http, send_message, None).await
            }

            TextBotResult::NotMatch => {}
        };
    }

    async fn ready(&self, _: Context, _: Ready) {
        log::info!("Discord Bot is ready!");
    }
}

async fn say(
    channel_id: ChannelId,
    http: &Arc<Http>,
    msg: impl Display,
    files: Option<Vec<FileEntry>>,
) {
    let result = match files {
        Some(mut files) => {
            let files = files.drain(..).map(|x| AttachmentType::Bytes {
                data: Cow::from(x.data),
                filename: x.name,
            });

            channel_id
                .send_files(&http, files, |e| e.content(msg))
                .await
        }

        None => channel_id.say(&http, msg).await,
    };

    if let Err(e) = result {
        log::warn!(
            "{:?}",
            anyhow::Error::new(e).context("Failed to send message to discord")
        );
    }
}
