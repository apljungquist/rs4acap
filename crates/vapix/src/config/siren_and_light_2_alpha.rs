//! The Siren and Light Configuration 2 API (alpha)

use reqwest::Method;
use serde_json::json;

use crate::{http::Request, rest_http2::RestHttp2};

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
}

impl RestHttp2 for GetMaintenanceModeRequest {
    type ResponseData = MaintenanceModeData;

    fn to_request(self) -> Request {
        Request::no_content(Method::GET, format!("{BASE_PATH}/maintenanceMode"))
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
}

impl RestHttp2 for StartMaintenanceModeRequest {
    type ResponseData = EmptyData;

    fn to_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because `self.data` can always be serialized to JSON.
        Request::json(Method::POST, format!("{BASE_PATH}/maintenanceMode/start"))
            .body(serde_json::to_string_pretty(&json!({"data": self.data})).unwrap())
    }
}

#[derive(Clone, Debug, Default)]
pub struct StopMaintenanceModeRequest;

impl StopMaintenanceModeRequest {
    pub fn new() -> Self {
        Self
    }
}

impl RestHttp2 for StopMaintenanceModeRequest {
    type ResponseData = EmptyData;

    fn to_request(self) -> Request {
        // PANICS:
        // The `unwrap` will never panic because the body is a static JSON value.
        Request::json(Method::POST, format!("{BASE_PATH}/maintenanceMode/stop"))
            .body(serde_json::to_string_pretty(&json!({"data": {}})).unwrap())
    }
}
