use super::content::ContentType;
use crate::api::requests::ApiRequest;
use reqwest::multipart::{Form, Part};
use reqwest::{Client, RequestBuilder, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

#[derive(Debug)]
pub struct PushContent {
    pub host_url: String,
    pub metadata_id: Uuid,
    pub content: ContentType,
    pub content_len: u64,
    pub content_hash: String,
}

#[derive(Debug, Serialize)]
struct PushContentData {
    pub metadata_id: Uuid,
    pub content_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct PushContentResponse {}

impl ApiRequest for PushContent {
    type ResponseType = PushContentResponse;
    type ErrorType = PushContentError;

    fn build_request(self, _base_url: &Url, client: &Client) -> RequestBuilder {
        let path = "/api/v1/upload".to_string();
        let full_url = Url::parse(&self.host_url).unwrap().join(&path).unwrap();

        // Create our form data
        let pc_req = PushContentData {
            metadata_id: self.metadata_id,
            content_hash: self.content_hash,
        };

        // Attach the form data to the request as json
        let multipart_json_data = serde_json::to_string(&pc_req).unwrap();
        let multipart_json = Part::bytes(multipart_json_data.as_bytes().to_vec())
            .mime_str("application/json")
            .unwrap();

        // Attach the CAR file to the request
        let multipart_car = Part::stream(self.content)
            .mime_str("application/vnd.ipld.car; version=2")
            .unwrap();

        // Combine the two parts into a multipart form
        let multipart_form = Form::new()
            .part("request-data", multipart_json)
            .part("car-upload", multipart_car);

        // post
        client
            .post(full_url)
            .multipart(multipart_form)
            .header(reqwest::header::CONTENT_LENGTH, self.content_len + 546)
    }

    fn requires_authentication(&self) -> bool {
        true
    }
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct PushContentError {
    #[serde(rename = "msg")]
    message: String,
}

impl Display for PushContentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(self.message.as_ref())
    }
}

impl Error for PushContentError {}
