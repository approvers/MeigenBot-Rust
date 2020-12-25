mod interaction;
mod verify;

use {
    anyhow::{Context, Result},
    interaction::on_interaction,
    serde_json::json,
    std::net::SocketAddr,
    warp::{
        reject::Reject,
        reply::{json as reply_json, Json},
        Filter, Rejection,
    },
};

pub struct DiscordWebhookServerOptions {
    pub token: String,
    pub app_public_key: String,
}

impl DiscordWebhookServerOptions {
    pub fn into_server(self) -> Result<DiscordWebhookServer> {
        let bytes = hex::decode(self.app_public_key)
            .context("Failed to parse app_public_key into bytes")?;

        Ok(DiscordWebhookServer {
            token: self.token,
            app_public_key_bytes: bytes,
        })
    }
}

pub struct DiscordWebhookServer {
    token: String,
    app_public_key_bytes: Vec<u8>,
}

impl DiscordWebhookServer {
    pub async fn start(self, ip: impl Into<SocketAddr>) -> Result<()> {
        let route = warp::post()
            .and(verify::filter(self.app_public_key_bytes))
            .and_then(on_request)
            .with(warp::log("discord_webhook_server"));

        warp::serve(route).run(ip.into()).await;
        Ok(())
    }
}

#[derive(Debug)]
struct JsonDeserializeError;
impl Reject for JsonDeserializeError {}

#[derive(Debug)]
struct UnknownEventType;
impl Reject for UnknownEventType {}

async fn on_request(body: String) -> Result<Json, Rejection> {
    #[derive(serde::Deserialize)]
    struct DiscordRequest {
        #[serde(rename = "type")]
        type_: u8,
    }

    let event = serde_json::from_str::<DiscordRequest>(&body)
        .map_err(|_| warp::reject::custom(JsonDeserializeError))?;

    match event.type_ {
        // ping
        1 => {
            log::info!("Discord Ping!");
            Ok(reply_json(&json!({ "type": 1 })))
        }

        // interaction
        2 => on_interaction(body).await,

        // ???
        _ => Err(warp::reject::custom(UnknownEventType)),
    }
}
