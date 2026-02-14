//! The [Recording group API].
//!
//! [Recording group API]: https://developer.axis.com/vapix/device-configuration/recording-group/

use anyhow::{ensure, Context};
use reqwest::Method;
use serde::{Deserialize, Serialize};

use crate::{rest::parse_data, rest_http::RestHttp, Client};

// TODO: Consider supporting v2beta, either inline or as a separate module.
const BASE_PATH: &str = "config/rest/recording-group/v2/recordingGroups";

/// Identifier (1–50 chars).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RecordingGroupId(String);

impl RecordingGroupId {
    pub fn try_new(id: String) -> anyhow::Result<Self> {
        ensure!(!id.is_empty(), "Recording group ID must not be empty");
        ensure!(
            id.len() <= 50,
            "Recording group ID must be at most 50 characters"
        );
        Ok(Self(id))
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for RecordingGroupId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ContainerFormat {
    Matroska,
    Cmaf,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum ProtectionScheme {
    CENC,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TargetAndMax {
    pub target: u64,
    pub max: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteObjectStorage {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postfix: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub remote_object_storage: RemoteObjectStorage,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicKey {
    pub key: String,
    pub key_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContentEncryption {
    pub key: String,
    pub key_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KeyEncryption {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificate_ids: Option<Vec<String>>,
    pub key_rotation_duration: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_keys: Option<Vec<PublicKey>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Encryption {
    Content {
        content_encryption: ContentEncryption,
        protection_scheme: ProtectionScheme,
    },
    Key {
        key_encryption: KeyEncryption,
        protection_scheme: ProtectionScheme,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncryptionWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    content_encryption: Option<ContentEncryption>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_encryption: Option<KeyEncryption>,
    protection_scheme: ProtectionScheme,
}

impl Serialize for Encryption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let wire = match self {
            Encryption::Content {
                content_encryption,
                protection_scheme,
            } => EncryptionWire {
                content_encryption: Some(content_encryption.clone()),
                key_encryption: None,
                protection_scheme: protection_scheme.clone(),
            },
            Encryption::Key {
                key_encryption,
                protection_scheme,
            } => EncryptionWire {
                content_encryption: None,
                key_encryption: Some(key_encryption.clone()),
                protection_scheme: protection_scheme.clone(),
            },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Encryption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = EncryptionWire::deserialize(deserializer)?;
        match (wire.content_encryption, wire.key_encryption) {
            (Some(content_encryption), None) => Ok(Encryption::Content {
                content_encryption,
                protection_scheme: wire.protection_scheme,
            }),
            (None, Some(key_encryption)) => Ok(Encryption::Key {
                key_encryption,
                protection_scheme: wire.protection_scheme,
            }),
            (Some(_), Some(_)) => Err(serde::de::Error::custom(
                "expected exactly one of contentEncryption or keyEncryption, got both",
            )),
            (None, None) => Err(serde::de::Error::custom(
                "expected exactly one of contentEncryption or keyEncryption, got neither",
            )),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingGroup {
    pub id: RecordingGroupId,
    pub nice_name: String,
    pub description: String,
    pub container_format: ContainerFormat,
    pub max_retention_time: u64,
    pub span_duration: u64,
    pub segment_duration: TargetAndMax,
    pub segment_size: TargetAndMax,
    pub pre_duration: u64,
    pub post_duration: u64,
    pub stream_options: String,
    #[serde(default)]
    pub encryption: Option<Encryption>,
    pub destinations: Vec<Destination>,
}

// TODO: Find a way to document the defaults that are easily discovered in the docs and in the IDE.
#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InputRecordingGroup {
    /// Auto-generated if omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<RecordingGroupId>,
    /// Human-readable display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) nice_name: Option<String>,
    /// User-provided group description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// Output format of recordings. Default: `Matroska`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) container_format: Option<ContainerFormat>,
    /// Max hours recordings are stored. Default: 0 (unlimited).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_retention_time: Option<u64>,
    /// Span duration in seconds. Default: 3600.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) span_duration: Option<u64>,
    /// Target/max segment duration in seconds. Default: 15/30.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) segment_duration: Option<TargetAndMax>,
    /// Target/max segment size in bytes. Default: 15 MB / 25 MB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) segment_size: Option<TargetAndMax>,
    /// Pre-trigger duration in milliseconds. Default: 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pre_duration: Option<u64>,
    /// Post-trigger duration in milliseconds. Default: 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) post_duration: Option<u64>,
    /// Stream parameters (camera, codec, fps, resolution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stream_options: Option<String>,
    /// Encryption configuration. Only supported with CMAF container format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) encryption: Option<Encryption>,
    /// Exactly one destination required.
    pub(crate) destinations: Vec<Destination>,
}

pub struct CreateRecordingGroupRequest {
    data: InputRecordingGroup,
}

impl CreateRecordingGroupRequest {
    pub fn id(mut self, id: RecordingGroupId) -> Self {
        self.data.id = Some(id);
        self
    }

    pub fn nice_name(mut self, nice_name: impl ToString) -> Self {
        self.data.nice_name = Some(nice_name.to_string());
        self
    }

    pub fn description(mut self, description: impl ToString) -> Self {
        self.data.description = Some(description.to_string());
        self
    }

    pub fn container_format(mut self, container_format: ContainerFormat) -> Self {
        self.data.container_format = Some(container_format);
        self
    }

    pub fn max_retention_time(mut self, hours: u64) -> Self {
        self.data.max_retention_time = Some(hours);
        self
    }

    pub fn span_duration(mut self, seconds: u64) -> Self {
        self.data.span_duration = Some(seconds);
        self
    }

    pub fn segment_duration(mut self, target_and_max: TargetAndMax) -> Self {
        self.data.segment_duration = Some(target_and_max);
        self
    }

    pub fn segment_size(mut self, target_and_max: TargetAndMax) -> Self {
        self.data.segment_size = Some(target_and_max);
        self
    }

    pub fn pre_duration(mut self, milliseconds: u64) -> Self {
        self.data.pre_duration = Some(milliseconds);
        self
    }

    pub fn post_duration(mut self, milliseconds: u64) -> Self {
        self.data.post_duration = Some(milliseconds);
        self
    }

    pub fn stream_options(mut self, stream_options: impl ToString) -> Self {
        self.data.stream_options = Some(stream_options.to_string());
        self
    }

    pub fn encryption(mut self, encryption: Encryption) -> Self {
        self.data.encryption = Some(encryption);
        self
    }
}

impl RestHttp for CreateRecordingGroupRequest {
    type RequestData = InputRecordingGroup;
    type ResponseData = RecordingGroup;
    const METHOD: Method = Method::POST;

    fn to_path_and_data(self) -> anyhow::Result<(String, Self::RequestData)> {
        Ok((BASE_PATH.to_string(), self.data))
    }
}

pub struct ListRecordingGroupsRequest;

impl ListRecordingGroupsRequest {
    pub async fn send(self, client: &Client) -> anyhow::Result<Vec<RecordingGroup>> {
        if cfg!(debug_assertions) {
            println!("Sending GET to {BASE_PATH}");
        }
        let response = client.request(Method::GET, BASE_PATH)?.send().await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("Could not fetch text, status was {status}"))?;
        if cfg!(debug_assertions) {
            println!("Received {status}: {text}");
        }
        parse_data(&text)
            .with_context(|| format!("Could not parse response as data; status: {status}."))
    }
}

pub struct GetRecordingGroupRequest {
    id: RecordingGroupId,
}

impl GetRecordingGroupRequest {
    pub async fn send(self, client: &Client) -> anyhow::Result<RecordingGroup> {
        let path = format!("{BASE_PATH}/{}", self.id.as_ref());
        if cfg!(debug_assertions) {
            println!("Sending GET to: {path}");
        }
        let response = client.request(Method::GET, &path)?.send().await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("Could not fetch text, status was {status}"))?;
        if cfg!(debug_assertions) {
            println!("Received {status}: {text}");
        }
        parse_data(&text)
            .with_context(|| format!("Could not parse response as data; status: {status}."))
    }
}

pub struct DeleteRecordingGroupRequest {
    id: RecordingGroupId,
}

impl DeleteRecordingGroupRequest {
    pub async fn send(self, client: &Client) -> anyhow::Result<()> {
        let path = format!("{BASE_PATH}/{}", self.id.as_ref());
        if cfg!(debug_assertions) {
            println!("Sending DELETE to: {path}");
        }
        let response = client.request(Method::DELETE, &path)?.send().await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("Could not fetch text, status was {status}"))?;
        if cfg!(debug_assertions) {
            println!("Received {status}: {text}");
        }
        let _: serde_json::Value = parse_data(&text)
            .with_context(|| format!("Could not parse response as data; status: {status}."))?;
        Ok(())
    }
}

pub fn list() -> ListRecordingGroupsRequest {
    ListRecordingGroupsRequest
}

pub fn get(id: RecordingGroupId) -> GetRecordingGroupRequest {
    GetRecordingGroupRequest { id }
}

pub fn create(destination_id: impl ToString) -> CreateRecordingGroupRequest {
    CreateRecordingGroupRequest {
        data: InputRecordingGroup {
            id: None,
            nice_name: None,
            description: None,
            container_format: None,
            max_retention_time: None,
            span_duration: None,
            segment_duration: None,
            segment_size: None,
            pre_duration: None,
            post_duration: None,
            stream_options: None,
            encryption: None,
            destinations: vec![Destination {
                remote_object_storage: RemoteObjectStorage {
                    id: destination_id.to_string(),
                    prefix: None,
                    postfix: None,
                },
            }],
        },
    }
}

pub fn delete(id: RecordingGroupId) -> DeleteRecordingGroupRequest {
    DeleteRecordingGroupRequest { id }
}
