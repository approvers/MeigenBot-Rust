use {
    super::DiscordRequest,
    ring::signature::{UnparsedPublicKey, ED25519},
    std::sync::Arc,
    warp::{
        hyper::body::Bytes,
        reject::{custom as reject_custom, Reject},
        Filter, Rejection,
    },
};

#[derive(Debug)]
struct SignatureVerifyError;
impl Reject for SignatureVerifyError {}

#[derive(Debug)]
struct JsonDecodeError;
impl Reject for JsonDecodeError {}

async fn verify_signature(
    key: Arc<Vec<u8>>,
    signature: String,
    timestamp: String,
    body: Bytes,
) -> Result<DiscordRequest, Rejection> {
    let signature = hex::decode(&signature).map_err(|_| reject_custom(SignatureVerifyError))?;

    let body = String::from_utf8(body.to_vec()).expect("Failed to parse as utf-8");
    let data = format!("{}{}", timestamp, body);

    UnparsedPublicKey::new(&ED25519, key.as_slice())
        .verify(data.as_bytes(), &signature)
        .map_err(|_| reject_custom(SignatureVerifyError))?;

    serde_json::from_str(&body).map_err(|_| reject_custom(JsonDecodeError))
}

pub(super) fn filter(
    public_key_bytes: Vec<u8>,
) -> impl Filter<Extract = (DiscordRequest,), Error = Rejection> + Clone {
    let public_key_bytes = Arc::new(public_key_bytes);

    warp::any()
        .and(warp::any().map(move || Arc::clone(&public_key_bytes)))
        .and(warp::header::<String>("X-Signature-Ed25519"))
        .and(warp::header::<String>("X-Signature-Timestamp"))
        .and(warp::filters::body::bytes())
        .and_then(verify_signature)
}
