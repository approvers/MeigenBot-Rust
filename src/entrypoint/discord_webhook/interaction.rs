use {
    super::JsonDeserializeError,
    crate::{command, db::MeigenDatabase, Synced},
    serde::{de::DeserializeOwned, Deserialize},
    serde_json::json,
    warp::{
        reject::{custom as custom_reject, Rejection},
        reply::{json as reply_json, Json},
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

pub(super) async fn on_interaction(
    body: String,
    db: Synced<impl MeigenDatabase>,
) -> Result<Json, Rejection> {
    let request = try_parse::<Request>(&body)?;

    if let Some(first_opt) = request.data.options.first() {
        let cmd_result = match first_opt.name.as_str() {
            "help" => Some(command::help().await),
            "status" => Some(command::status(db).await),
            _ => None,
        };

        if let Some(cmd_result) = cmd_result {
            let msg = match cmd_result {
                Ok(v) => v,
                Err(e) => {
                    log::error!("something went wrong: {:?}", e);
                    String::from(
                        "処理がうまくいきませんでした。 <@391857452360007680> ログを見てください。",
                    )
                }
            };

            return Ok(reply_json(&json!({
                // ChannelMessageWithSource: respond with a message, showing the user's input
                "type": 4,
                "data": {
                    "content": msg
                }
            })));
        }
    }

    unimplemented!()
}
