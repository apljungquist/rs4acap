//! The Siren and Light Configuration 2 API (alpha)

use reqwest::Method;
use serde_json::json;

use crate::{
    http::{Error, HttpClient, Request},
    rest, rest_http,
};

const BASE_PATH: &str = "config/rest/siren-and-light/v2alpha";

// Objects (used only by responses)

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct EmptyData {}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceModeData {
    pub running: Option<bool>,
    pub supported: Option<bool>,
}

// Objects (used only by requests)

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct StartMaintenanceModeData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time: Option<i64>,
}

// Requests

#[derive(Clone, Debug, Default)]
pub struct GetMaintenanceModeRequest;

impl GetMaintenanceModeRequest {
    pub fn new() -> Self {
        Self
    }

    pub fn into_request(self) -> Request {
        Request::new(Method::GET, format!("{BASE_PATH}/maintenanceMode"))
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<MaintenanceModeData, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}

#[derive(Clone, Debug, Default)]
pub struct StartMaintenanceModeRequest {
    data: StartMaintenanceModeData,
}

impl StartMaintenanceModeRequest {
    pub fn new() -> Self {
        Self {
            data: StartMaintenanceModeData::default(),
        }
    }

    pub fn into_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because `self.data` can always be serialized to JSON.
        Request::new(Method::POST, format!("{BASE_PATH}/maintenanceMode/start"))
            .json(serde_json::to_string_pretty(&json!({"data": self.data})).unwrap())
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<EmptyData, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopMaintenanceModeRequest;

impl StopMaintenanceModeRequest {
    pub fn new() -> Self {
        Self
    }

    pub fn into_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because the body is a static JSON value.
        Request::new(Method::POST, format!("{BASE_PATH}/maintenanceMode/stop"))
            .json(serde_json::to_string_pretty(&json!({"data": {}})).unwrap())
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<EmptyData, Error<rest::Error>> {
        rest_http::send_request(client, self.into_request()).await
    }
}
