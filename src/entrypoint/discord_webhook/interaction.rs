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

enum RunCommandError {
    InternalServerError(anyhow::Error),
    InvalidRequest(&'static str),
}

async fn run_command(
    req: &Request,
    db: Synced<impl MeigenDatabase>,
) -> Result<String, RunCommandError> {
    use {crate::command::*, RunCommandError::*};

    let first_opt = req
        .data
        .options
        .first()
        .ok_or(InvalidRequest("meigen command requires subcommand"))?;

    fn get<'a>(opt: &'a RequestOption, key: &str) -> Option<&'a String> {
        opt.options
            .as_ref()?
            .iter()
            .find(|x| x.name == key)?
            .value
            .as_ref()
    }

    fn on_parse_fail(
        field_name: &'static str,
        ty: &'static str,
    ) -> Result<String, RunCommandError> {
        Ok(format!("{}フィールド({})のパースに失敗しました。もしかしたら数字が大きすぎるとか小さすぎるとかマイナスだからとかかもしれません。", field_name, ty))
    }

    macro_rules! extract {
        ({
            from: $from:ident
            $(,required: [$($required_field:ident$(:$required_field_type:ty)?),+$(,)?])?
            $(,optional: [$($optional_field:ident$(:$optional_field_type:ty)?),+$(,)?])?
            $(,)?
        }) => {{
            $($(let $required_field = match get($from, stringify!($required_field)) {
                Some(value) => {
                    $(let value = match value.parse::<$required_field_type>() {
                        Ok(v) => v,
                        Err(_) => return on_parse_fail(stringify!($required_field), stringify!($required_field_type))
                    };)?
                    value
                },
                None => {
                    return Err(InvalidRequest(
                        concat!(stringify!($required_field), " field is missing")
                    ))
                }
            };)+)?
            $($(let $optional_field = match get($from, stringify!($optional_field)) {
                Some(value) => {
                    $(let value = match value.parse::<$optional_field_type>() {
                        Ok(v) => v,
                        Err(_) => return on_parse_fail(stringify!($optional_field), stringify!($optional_field_type))
                    };)?
                    Some(value)
                },
                None => None,
            };)+)?

            (($($($required_field),+)?),($($($optional_field),+)?))
        }};
    }

    match first_opt.name.as_str() {
        "make" => {
            let ((author, content), ()) = extract!({
                from: first_opt,
                required: [author, content]
            });

            make(db, author, content).await
        }

        "search" => {
            let sub = first_opt
                .options
                .as_ref()
                .and_then(|x| x.first())
                .ok_or_else(|| {
                    RunCommandError::InvalidRequest("search command requires subcommand")
                })?;

            match sub.name.as_str() {
                "author" => {
                    let (author, (show_count, page)) = extract!({
                        from: sub,
                        required: [author],
                        optional: [show_count: u8, page: u32]
                    });

                    search_author(db, author, show_count, page).await
                }

                "content" => {
                    let (content, (show_count, page)) = extract!({
                        from: sub,
                        required: [content],
                        optional: [show_count: u8, page: u32]
                    });

                    search_content(db, content, show_count, page).await
                }
                _ => return Err(InvalidRequest("unexpected subcommand for search command")),
            }
        }

        "help" => help().await,
        "id" => {
            let (req_id, ()) = extract!({
                from: first_opt,
                required: [id: u32],
            });

            id(db, req_id).await
        }
        "list" => {
            let ((), (count, page)) = extract!({
                from: first_opt,
                optional: [count: u8, page: u32],
            });

            list(db, count, page).await
        }
        "random" => {
            let ((), count) = extract!({
                from: first_opt,
                optional: [count: u8],
            });

            random(db, count).await
        }
        "status" => status(db).await,
        "delete" => {
            let (count, ()) = extract!({
                from: first_opt,
                required: [count: u32],
            });

            // TODO: kawaemon restrict
            delete(db, count).await
        }
        _ => return Err(InvalidRequest("unexpected subcommand")),
    }
    .map_err(InternalServerError)
}
