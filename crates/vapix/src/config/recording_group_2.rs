//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/

use reqwest::Method;
use serde_json::json;

use crate::{
    cassette::Request, remote_object_storage_1_beta::DestinationId, rest_http2::RestHttp2,
};

const BASE_PATH: &str = "config/rest/recording-group/v2/recordingGroups";

// Scalars

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RecordingGroupId(String);

impl RecordingGroupId {
    pub fn new(id: String) -> Self {
        Self(id)
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ContainerFormat {
    Matroska,
    Cmaf,
}

// Objects (used by both requests and responses)

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEncryption {
    pub key: String,
    pub key_id: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub key: String,
    pub key_id: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyEncryption {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
    pub key_rotation_duration: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Encryption {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_encryption: Option<ContentEncryption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_encryption: Option<KeyEncryption>,
    pub protection_scheme: String,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentDuration {
    pub target: u64,
    pub max: u64,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentSize {
    pub target: u64,
    pub max: u64,
}

// Objects (used only by responses)

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_object_storage: Option<RemoteObjectStorage>,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObjectStorage {
    pub id: DestinationId,
    pub prefix: String,
    pub postfix: String,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordingGroupData {
    pub id: RecordingGroupId,
    pub container_format: ContainerFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub nice_name: String,
    pub destinations: Vec<Destination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<Encryption>,
    pub max_retention_time: u64,
    pub pre_duration: u64,
    pub post_duration: u64,
    pub span_duration: u64,
    pub segment_duration: SegmentDuration,
    pub segment_size: SegmentSize,
    pub stream_options: String,
}

// Objects (used only by requests)

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputDestination {
    pub remote_object_storage: InputRemoteObjectStorage,
}

impl InputDestination {
    fn could_create(&self, other: &Destination) -> bool {
        let InputRemoteObjectStorage {
            id,
            prefix,
            postfix,
        } = &self.remote_object_storage;

        let Some(ref other) = other.remote_object_storage else {
            return false;
        };
        if id != &other.id {
            return false;
        }
        if let (Some(old), new) = (prefix, &other.prefix) {
            if old != new {
                return false;
            }
        }
        if let (Some(old), new) = (postfix, &other.postfix) {
            if old != new {}
        }

        true
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputRemoteObjectStorage {
    pub id: DestinationId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postfix: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRecordingGroupData {
    pub destinations: Vec<InputDestination>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_format: Option<ContainerFormat>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nice_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encryption: Option<Encryption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retention_time: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_duration: Option<SegmentDuration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_size: Option<SegmentSize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<String>,
}

impl CreateRecordingGroupData {
    /// Returns true if the request recreates the given recording group, ignoring the ID.
    ///
    /// Note that any fields that are not nullable in the response type and are unset in the
    /// request are assumed to be the default value and compare equivalent.
    pub fn could_create(&self, other: &RecordingGroupData) -> bool {
        let CreateRecordingGroupData {
            destinations,
            container_format,
            description,
            nice_name,
            encryption,
            max_retention_time,
            pre_duration,
            post_duration,
            span_duration,
            segment_duration,
            segment_size,
            stream_options,
        } = self;

        if other.destinations.len() != destinations.len() {
            return false;
        }
        for (old, new) in other.destinations.iter().zip(destinations) {
            if !new.could_create(old) {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.container_format, &container_format) {
            if old != new {
                return false;
            }
        }

        if let (Some(old), Some(new)) = (&other.description, &description) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.nice_name, &nice_name) {
            if old != new {
                return false;
            }
        }

        if let (Some(old), Some(new)) = (&other.encryption, &encryption) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.max_retention_time, &max_retention_time) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.pre_duration, &pre_duration) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.post_duration, &post_duration) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.span_duration, &span_duration) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.segment_duration, &segment_duration) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.segment_size, &segment_size) {
            if old != new {
                return false;
            }
        }

        if let (old, Some(new)) = (&other.stream_options, &stream_options) {
            if old != new {
                return false;
            }
        }

        true
    }
}

// Requests

#[derive(Debug)]
pub struct CreateRecordingGroupRequest {
    data: CreateRecordingGroupData,
}

impl CreateRecordingGroupRequest {
    pub fn from_data(data: CreateRecordingGroupData) -> Self {
        Self { data }
    }

    pub fn remote_object_storage(id: DestinationId) -> Self {
        Self {
            data: CreateRecordingGroupData {
                destinations: vec![InputDestination {
                    remote_object_storage: InputRemoteObjectStorage {
                        id,
                        prefix: None,
                        postfix: None,
                    },
                }],
                container_format: None,
                description: None,
                nice_name: None,
                encryption: None,
                max_retention_time: None,
                pre_duration: None,
                post_duration: None,
                span_duration: None,
                segment_duration: None,
                segment_size: None,
                stream_options: None,
            },
        }
    }

    pub fn description(mut self, description: String) -> Self {
        self.data.description = Some(description);
        self
    }
}

impl RestHttp2 for CreateRecordingGroupRequest {
    type ResponseData = RecordingGroupData;

    fn to_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because `self.data` can always be serialized to JSON.
        Request::json(Method::POST, BASE_PATH.to_string())
            .body(serde_json::to_string_pretty(&json!({"data":self.data})).unwrap())
    }
}

#[derive(Debug, Default)]
pub struct ListRecordingGroupsRequest;

impl ListRecordingGroupsRequest {
    pub fn new() -> Self {
        Self
    }
}

impl RestHttp2 for ListRecordingGroupsRequest {
    type ResponseData = Vec<RecordingGroupData>;

    fn to_request(self) -> Request {
        Request::no_content(Method::GET, BASE_PATH.to_string())
    }
}

#[derive(Debug)]
pub struct DeleteRecordingGroupRequest {
    id: RecordingGroupId,
}

impl DeleteRecordingGroupRequest {
    pub fn new(id: RecordingGroupId) -> Self {
        Self { id }
    }
}

impl RestHttp2 for DeleteRecordingGroupRequest {
    type ResponseData = ();

    fn to_request(self) -> Request {
        Request::no_content(
            Method::DELETE,
            format!("{BASE_PATH}/{}", self.id.into_string()),
        )
    }
}
