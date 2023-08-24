use super::{
    error::ClientError,
    requests::{ApiRequest, StreamableApiRequest},
};
use anyhow::Result;
use bytes::Bytes;
use chrono::naive::serde::ts_microseconds::deserialize;
use futures_core::stream::Stream;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client as ReqwestClient, Url,
};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use tokio::runtime::Runtime;
use tomb_crypt::prelude::*;
use uuid::Uuid;

#[derive(Clone)]
/// Credentials in order to sign and verify messages for a Banyan account
pub struct Credentials {
    /// The unique account id (used as a JWT subject)
    pub account_id: Uuid,
    /// The signing key (used to sign JWTs)
    pub signing_key: EcSignatureKey,
}

impl Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Get the pem string for the signing key
        f.debug_struct("Credentials")
            .field("account_id", &self.account_id)
            .finish()
    }
}

/// The audience for the API token
const AUDIENCE: &str = "banyan-platform";

#[derive(Debug, Clone)]
/// Client for interacting with our API
pub struct Client {
    /// Base URL
    pub remote: Url,
    /// Bearer auth
    pub claims: Option<ApiToken>,
    /// Credentials for signing
    pub signing_key: Option<EcSignatureKey>,
    /// The current bearer token
    pub bearer_token: Option<String>,
    /// The reqwest client
    reqwest_client: ReqwestClient,
}

impl Client {
    /// Create a new Client at a remote endpoint
    /// # Arguments
    /// * `remote` - The base URL for the API
    /// # Returns
    /// * `Self` - The client
    pub fn new(remote: &str) -> Result<Self> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
        let reqwest_client = ReqwestClient::builder()
            .default_headers(default_headers)
            .build()
            .unwrap();

        Ok(Self {
            remote: Url::parse(remote)?,
            claims: None,
            signing_key: None,
            bearer_token: None,
            reqwest_client,
        })
    }

    /// Set the credentials for signing
    /// # Arguments
    /// * `credentials` - The credentials to use for signing
    pub fn with_credentials(&mut self, credentials: Credentials) {
        self.bearer_token = None;
        self.claims = Some(ApiToken::new(
            AUDIENCE.to_string(),
            credentials.account_id.to_string(),
        ));
        self.signing_key = Some(credentials.signing_key);
    }

    /// Set the bearer token directly
    /// # Arguments
    /// * `bearer_token` - The bearer token to use
    pub fn with_bearer_token(&mut self, bearer_token: String) {
        self.claims = None;
        self.signing_key = None;
        self.bearer_token = Some(bearer_token);
    }

    /// Return a bearer token based on the current credentials
    /// # Returns
    /// * `Option<String>` - The bearer token
    /// # Errors
    /// * `ClientError` - If there is an error generating the token.
    ///    If the bearer token can not be encoded, or if the signing key is not available.
    pub async fn bearer_token(&mut self) -> Result<String, ClientError> {
        match &self.claims {
            Some(claims) => {
                let is_expired = claims.is_expired().map_err(ClientError::crypto_error)?;
                // If we already have a bearer token and the claims are still valid
                // return the current bearer token
                if !is_expired && self.bearer_token.is_some() {
                    return Ok(self.bearer_token.clone().unwrap());
                } else if is_expired {
                    claims.refresh().map_err(ClientError::crypto_error)?;
                }
                match &self.signing_key {
                    Some(signing_key) => {
                        self.bearer_token = Some(
                            claims
                                .encode_to(signing_key)
                                .await
                                .map_err(ClientError::crypto_error)?,
                        );
                        Ok(self.bearer_token.clone().unwrap())
                    }
                    _ => Err(ClientError::auth_unavailable()),
                }
            }
            // No claims, so no bearer token
            _ => match &self.bearer_token {
                Some(bearer_token) => Ok(bearer_token.clone()),
                _ => Err(ClientError::auth_unavailable()),
            },
        }
    }

    /// Get the current subject based on the set credentials
    pub fn subject(&self) -> Result<String, ClientError> {
        match &self.claims {
            Some(claims) => {
                let sub = claims.sub().map_err(ClientError::crypto_error)?;
                Ok(sub.to_string())
            }
            _ => Err(ClientError::auth_unavailable()),
        }
    }

    /// Call a method that implements ApiRequest
    pub async fn call<T: ApiRequest>(
        &mut self,
        request: T,
    ) -> Result<T::ResponseType, ClientError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder
            .send()
            .await
            .map_err(ClientError::http_error)?;

        if response.status().is_success() {
            response
                .json::<T::ResponseType>()
                .await
                .map_err(ClientError::bad_format)
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ClientError::http_response_error(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response
                .json::<T::ErrorType>()
                .await
                .map_err(ClientError::bad_format)?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ClientError::from(err))
        }
    }

    /// Stream a response from the API that implements StreamableApiRequest
    pub async fn stream<T: StreamableApiRequest>(
        &mut self,
        request: T,
    ) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, ClientError> {
        let add_authentication = request.requires_authentication();
        let mut request_builder = request.build_request(&self.remote, &self.reqwest_client);
        if add_authentication {
            let bearer_token = self.bearer_token().await?;
            request_builder = request_builder.bearer_auth(bearer_token);
        }

        let response = request_builder
            .send()
            .await
            .map_err(ClientError::http_error)?;

        if response.status().is_success() {
            Ok(response.bytes_stream())
        } else {
            if response.status() == reqwest::StatusCode::NOT_FOUND {
                // Handle 404 specifically
                // You can extend this part to handle other status codes differently if needed
                return Err(ClientError::http_response_error(response.status()));
            }
            // For other error responses, try to deserialize the error
            let err = response
                .json::<T::ErrorType>()
                .await
                .map_err(ClientError::bad_format)?;

            let err = Box::new(err) as Box<dyn std::error::Error + Send + Sync + 'static>;
            Err(ClientError::from(err))
        }
    }
}

impl Serialize for Client {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let remote = self.remote.as_str().to_string();
        
        let key = if let Some(key) = self.signing_key.as_ref() {
            // Create a runtime from which to run the aynchronous export code
            let key_bytes = Runtime::new()
                .expect("failed to create new runtime")
                .block_on(async { key.export().await.expect("failed to export key") });

            Some(key_bytes)
        } else { None };

        (remote, &self.claims, key, &self.bearer_token).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Client {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (remote, claims, key, bearer_token) =
            <(String, Option<ApiToken>, Option<Vec<u8>>, Option<String>)>::deserialize(deserializer)?;
        // Create a new client
        let mut client =
            Self::new(&remote).expect("failed to create new client with endpoint in deserialize");
        // Set the claims
        client.claims = claims;

        if let Some(key_bytes) = key {
            // Create a runtime from which to run the aynchronous export code
            let key = Runtime::new()
                .expect("failed to create new runtime")
                .block_on(async { EcSignatureKey::import(&key_bytes).await.expect("failed to import key") });
            client.signing_key = Some(key);
        }

        // If there is a bearer token
        if let Some(bearer_token) = bearer_token {
            // Set it
            client.with_bearer_token(bearer_token);
        }

        Ok(client)
    }
}
