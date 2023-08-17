use reqwest::{Client, RequestBuilder, Url};
use serde::Deserialize;
use std::error::Error;
use uuid::Uuid;

use crate::banyan::api::ApiRequest;

#[derive(Debug)]
pub struct ReadWhoAmI;

#[derive(Debug, Deserialize)]
pub struct ReadWhoAmIResponse {
    pub account_id: Uuid,
}

impl ApiRequest for ReadWhoAmI {
    type ErrorType = ReadWhoAmIError;
    type ResponseType = ReadWhoAmIResponse;

    fn build_request(self, base_url: &Url, client: &Client) -> RequestBuilder {
        let full_url = base_url.join("/api/v1/auth/who_am_i").unwrap();
        client.get(full_url)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ReadWhoAmIError {
    #[serde(rename = "error")]
    kind: ReadWhoAmIErrorKind,
}

#[derive(Debug, Deserialize)]
enum ReadWhoAmIErrorKind {
    Unknown,
}

impl Error for ReadWhoAmIError {}

impl std::fmt::Display for ReadWhoAmIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ReadWhoAmIErrorKind::*;

        let msg = match &self.kind {
            Unknown => "an unknown error occurred reading the account",
        };

        f.write_str(msg)
    }
}
