//! The [Remote Object Storage API] (beta).
//!
//! [Remote Object Storage API]: https://developer.axis.com/vapix/device-configuration/remote-object-storage-api/

use reqwest::Method;
use serde_json::json;
use url::Url;

use crate::{cassette::Request, rest_http2::RestHttp2};

const BASE_PATH: &str = "config/rest/remote-object-storage/v1beta/destinations";

// Scalars

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DestinationId(String);

impl DestinationId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

// Objects (used by both requests and responses)

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AzureDestination {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sas: Option<String>,
}

impl AzureDestination {
    pub fn new(container: String, sas: String, url: Url) -> Self {
        Self {
            account_name: None,
            container: Some(container),
            url: Some(url.to_string()),
            shared_key: None,
            sas: Some(sas),
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct S3Destination {
    pub bucket: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

// Objects (used only by responses)

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationData {
    pub id: DestinationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub azure: Option<AzureDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Destination>,
}

// Objects (used only by requests)

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDestinationData {
    id: DestinationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    azure: Option<AzureDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    s3: Option<S3Destination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

// Requests

#[derive(Debug)]
pub struct CreateDestinationRequest {
    data: CreateDestinationData,
}

impl CreateDestinationRequest {
    pub fn description(mut self, description: String) -> Self {
        self.data.description = Some(description);
        self
    }
}

impl CreateDestinationRequest {
    pub fn azure(id: DestinationId, azure: AzureDestination) -> Self {
        Self {
            data: CreateDestinationData {
                id,
                azure: Some(azure),
                s3: None,
                description: None,
            },
        }
    }

    pub fn s3(id: DestinationId, s3: S3Destination) -> Self {
        Self {
            data: CreateDestinationData {
                id,
                azure: None,
                s3: Some(s3),
                description: None,
            },
        }
    }
}

impl RestHttp2 for CreateDestinationRequest {
    type ResponseData = DestinationData;

    fn to_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because `self.data` can always be serialized to JSON.
        Request::new(Method::POST, BASE_PATH.to_string())
            .body(serde_json::to_string_pretty(&json!({"data":self.data})).unwrap())
    }
}

#[derive(Debug, Default)]
pub struct ListDestinationsRequest;

impl ListDestinationsRequest {
    pub fn new() -> Self {
        Self
    }
}

impl RestHttp2 for ListDestinationsRequest {
    type ResponseData = Vec<DestinationData>;

    fn to_request(self) -> Request {
        Request::new(Method::GET, BASE_PATH.to_string())
    }
}

#[derive(Debug)]
pub struct UpdateDestinationRequest {
    id: DestinationId,
    property: String,
    data: serde_json::Value,
}

impl UpdateDestinationRequest {
    pub fn description(id: DestinationId, description: String) -> Self {
        Self {
            id,
            property: "description".to_string(),
            data: serde_json::Value::String(description),
        }
    }
}

impl RestHttp2 for UpdateDestinationRequest {
    type ResponseData = ();

    fn to_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because `self.data` is a `serde_json::Value` which can
        // always be serialized to JSON.
        Request::new(
            Method::PATCH,
            format!("{BASE_PATH}/{}/{}", self.id.into_string(), self.property),
        )
        .body(serde_json::to_string_pretty(&json!({"data":self.data})).unwrap())
    }
}

#[derive(Debug)]
pub struct DeleteDestinationRequest {
    id: DestinationId,
}

impl DeleteDestinationRequest {
    pub fn new(id: DestinationId) -> Self {
        Self { id }
    }
}

impl RestHttp2 for DeleteDestinationRequest {
    type ResponseData = ();

    fn to_request(self) -> Request {
        Request::new(
            Method::DELETE,
            format!("{BASE_PATH}/{}", self.id.into_string()),
        )
    }
}
