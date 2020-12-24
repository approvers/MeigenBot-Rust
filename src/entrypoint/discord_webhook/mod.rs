mod verify;

use {
    anyhow::{Context, Result},
    std::net::SocketAddr,
    warp::{Filter, Rejection, Reply},
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

#[derive(serde::Deserialize)]
struct DiscordRequest {
    #[serde(rename = "type")]
    type_: u8,
}

async fn on_request(request: DiscordRequest) -> Result<impl Reply, Rejection> {
    use {serde_json::json, warp::reply::json as reply_json};

    match request.type_ {
        // ping
        1 => Ok(reply_json(&json!({ "type": 1 }))),
        _ => unimplemented!(),
    }
}
