use std::time::Duration;

use anyhow::Context as _;
use async_trait::async_trait;
use reqwest::StatusCode;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct Credential {
    user_id: String,
}

pub enum Error {
    Internal(anyhow::Error),
    InvalidToken,
}

#[async_trait]
pub trait Authenticator: Send + Sync + Clone + 'static {
    async fn auth(&self, token: &str) -> Result<Credential, Error>;
}

#[derive(Clone)]
pub struct GAuth<'a> {
    client: reqwest::Client,
    endpoint: &'a str,
}

impl<'a> GAuth<'a> {
    pub fn new(endpoint: &'a str) -> Self {
        Self {
            endpoint,
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap(),
        }
    }
}

#[async_trait]
impl Authenticator for GAuth<'static> {
    async fn auth(&self, token: &str) -> Result<Credential, Error> {
        let result = self
            .client
            .post(self.endpoint)
            .body(serde_json::json!({ "token": token }).to_string())
            .send()
            .await
            .context("failed to send auth request")
            .map_err(Error::Internal)?;

        let status = result.status();
        let body = result
            .text()
            .await
            .context("failed to receive body")
            .map_err(Error::Internal)?;

        match status {
            StatusCode::OK => {
                let response = serde_json::from_str(&body)
                    .context("failed to deserialize response json")
                    .map_err(Error::Internal)?;

                Ok(response)
            }

            StatusCode::UNAUTHORIZED => Err(Error::InvalidToken),

            p => Err(Error::Internal(anyhow::anyhow!(
                "auth request failed: status: {}, body: {}",
                p,
                body
            ))),
        }
    }
}

#[cfg(feature = "api_auth_always_pass")]
#[derive(Clone)]
pub struct AlwaysPass;

#[cfg(feature = "api_auth_always_pass")]
#[async_trait]
impl Authenticator for AlwaysPass {
    async fn auth(&self, _token: &str) -> Result<Credential, Error> {
        Ok(Credential {
            user_id: String::new(),
        })
    }
}
