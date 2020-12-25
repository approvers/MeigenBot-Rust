use std::unimplemented;

use {
    super::JsonDeserializeError,
    serde::{de::DeserializeOwned, Deserialize},
    warp::{
        reject::{custom as custom_reject, Rejection},
        reply::Json,
    },
};

#[derive(Deserialize)]
struct Request {
    data: RequestData,
}

#[derive(Deserialize)]
struct RequestData {
    options: Vec<RequestOption>,
}

#[derive(Deserialize)]
struct RequestOption {
    name: String,
    value: Option<String>,
    options: Option<Vec<RequestOption>>,
}

fn try_parse<T: DeserializeOwned>(data: &str) -> Result<T, Rejection> {
    serde_json::from_str(data).map_err(|_| custom_reject(JsonDeserializeError))
}

pub(super) async fn on_interaction(body: String) -> Result<Json, Rejection> {
    let request = try_parse::<Request>(&body)?;

    if let Some(first_opt) = request.data.options.first() {
        log::info!("command: {}", first_opt.name);
    }

    unimplemented!()
}
