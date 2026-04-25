//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    http::{Error, HttpClient, Request},
    rest, rest_http,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordingGroupResponse {
    pub id: String,
    pub container_format: String,
    pub description: String,
    pub destinations: Vec<Destination>,
    pub max_retention_time: u64,
    pub nice_name: String,
    pub post_duration: u64,
    pub pre_duration: u64,
    pub segment_duration: SegmentDuration,
    pub segment_size: SegmentSize,
    pub span_duration: u64,
    pub stream_options: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub remote_object_storage: RemoteObjectStorage,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObjectStorage {
    pub id: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub postfix: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SegmentDuration {
    pub max: u64,
    pub target: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SegmentSize {
    pub max: u64,
    pub target: u64,
}

#[derive(Debug, Default)]
pub struct CreateRecordingGroupsRequest {
    data: Value,
}

impl CreateRecordingGroupsRequest {
    pub fn new() -> Self {
        Self { data: Value::Null }
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub fn into_request(self) -> Request {
        let body = serde_json::to_string_pretty(&json!({"data": self.data})).unwrap();
        Request::new(
            Method::POST,
            "config/rest/recording-group/v2beta/recordingGroups".to_string(),
        )
        .json(body)
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<CreateRecordingGroupResponse, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}
