use std::sync::Arc;

use ring::signature::{UnparsedPublicKey, ED25519};
use warp::{
    http::StatusCode,
    hyper::body::Bytes,
    reject::{custom as reject_custom, Reject},
    reply::{with_status as reply_with_status, WithStatus},
    Filter, Rejection,
};

use super::inject;

pub(super) fn filter(
    public_key_bytes: Vec<u8>,
) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
    let public_key_bytes = Arc::new(public_key_bytes);

    warp::any()
        .and(inject(public_key_bytes))
        .and(warp::header::<String>("X-Signature-Ed25519"))
        .and(warp::header::<String>("X-Signature-Timestamp"))
        .and(warp::filters::body::bytes())
        .and_then(verify_signature)
}

pub(super) fn try_recover(err: &Rejection) -> Option<WithStatus<&'static str>> {
    if let Some(SignatureVerifyError) = err.find() {
        return Some(reply_with_status(
            "invalid request signature",
            StatusCode::UNAUTHORIZED,
        ));
    }

    None
}

#[derive(Debug)]
struct SignatureVerifyError;
impl Reject for SignatureVerifyError {}

async fn verify_signature(
    key: Arc<Vec<u8>>,
    signature: String,
    timestamp: String,
    body: Bytes,
) -> Result<String, Rejection> {
    let signature = hex::decode(&signature).map_err(|_| {
        tracing::trace!("failed to decode signature");
        reject_custom(SignatureVerifyError)
    })?;

    let body = String::from_utf8(body.to_vec()).expect("failed to parse as utf-8");
    let data = format!("{}{}", timestamp, body);

    UnparsedPublicKey::new(&ED25519, key.as_slice())
        .verify(data.as_bytes(), &signature)
        .map_err(|e| {
            tracing::trace!("failed to verify signature: {}", e);
            reject_custom(SignatureVerifyError)
        })?;

    tracing::trace!("no error reported while verifying signature");

    Ok(body)
}
