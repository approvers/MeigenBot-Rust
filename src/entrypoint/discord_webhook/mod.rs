mod interaction;
mod model;
mod verify;

use {
    crate::{db::MeigenDatabase, Synced},
    anyhow::{Context, Result},
    interaction::on_interaction,
    serde_json::json,
    std::{convert::Infallible, net::SocketAddr, sync::Arc},
    tokio::sync::RwLock,
    warp::{
        http::StatusCode,
        reject::Reject,
        reply::{json as reply_json, with_status as reply_with_status, Json},
        Filter, Rejection, Reply,
    },
};

// 512KB limit
const CONTENT_LENGTH_LIMIT: u64 = 1024 * 512;

// TODO: builder pattern is more rust-ish
pub struct DiscordWebhookServerOptions<D: MeigenDatabase> {
    pub app_public_key: String,
    pub db: D,
}

impl<D: MeigenDatabase> DiscordWebhookServerOptions<D> {
    pub fn into_server(self) -> Result<DiscordWebhookServer<D>> {
        let bytes = hex::decode(self.app_public_key)
            .context("Failed to parse app_public_key into bytes")?;

        Ok(DiscordWebhookServer {
            app_public_key_bytes: bytes,
            db: Arc::new(RwLock::new(self.db)),
        })
    }
}

pub struct DiscordWebhookServer<D: MeigenDatabase> {
    app_public_key_bytes: Vec<u8>,
    db: Synced<D>,
}

impl<D: MeigenDatabase> DiscordWebhookServer<D> {
    pub async fn start(self, ip: impl Into<SocketAddr>) -> Result<()> {
        let route = warp::post()
            .and(warp::body::content_length_limit(CONTENT_LENGTH_LIMIT))
            .and(verify::filter(self.app_public_key_bytes))
            .and(inject(self.db))
            .and_then(on_request)
            .recover(recover)
            .with(warp::log("discord_webhook_server"));

        warp::serve(route).run(ip.into()).await;
        Ok(())
    }
}

fn inject<T>(t: Arc<T>) -> impl Filter<Extract = (Arc<T>,), Error = Infallible> + Clone
where
    T: Send + Sync,
{
    warp::any().map(move || Arc::clone(&t))
}

macro_rules! warp_rejects {
    ($($name: ident),+$(,)?) => {
        $(#[derive(Debug)]
        struct $name;
        impl Reject for $name {})+
    };
}

warp_rejects! {
    JsonDeserializeError,
    UnknownEventType,
    InternalServerError,
    BadRequest,
}

async fn on_request(body: String, db: Synced<impl MeigenDatabase>) -> Result<Json, Rejection> {
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
        2 => on_interaction(body, db).await,

        // ???
        _ => Err(warp::reject::custom(UnknownEventType)),
    }
}

async fn recover(err: Rejection) -> Result<impl Reply, Rejection> {
    if let Some(reply) = verify::try_recover(&err) {
        return Ok(reply);
    }

    if let Some(JsonDeserializeError) = err.find() {
        return Ok(reply_with_status("invalid json", StatusCode::BAD_REQUEST));
    }

    if let Some(UnknownEventType) = err.find() {
        return Ok(reply_with_status(
            "unknown event type",
            StatusCode::BAD_REQUEST,
        ));
    }

    if let Some(InternalServerError) = err.find() {
        return Ok(reply_with_status(
            "internal server error",
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if let Some(BadRequest) = err.find() {
        return Ok(reply_with_status(
            "invalid request",
            StatusCode::BAD_REQUEST,
        ));
    }

    Err(err)
}
