//! The [Firmware Management API].
//!
//! [Firmware Management API]: https://developer.axis.com/vapix/network-video/firmware-management-api/

use serde::{Deserialize, Serialize};

use crate::json_rpc_http::JsonRpcHttp;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FactoryDefaultMode {
    Soft,
    Hard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FactoryDefaultParams {
    factory_default_mode: FactoryDefaultMode,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryDefaultRequest {
    api_version: &'static str,
    method: &'static str,
    params: FactoryDefaultParams,
}

#[derive(Debug, Deserialize)]
pub struct FactoryDefaultData {}

impl JsonRpcHttp for FactoryDefaultRequest {
    type Data = FactoryDefaultData;
    const PATH: &'static str = "axis-cgi/firmwaremanagement.cgi";
}

/// Create a factory default request.
pub fn factory_default(mode: FactoryDefaultMode) -> FactoryDefaultRequest {
    FactoryDefaultRequest {
        api_version: "1.0",
        method: "factoryDefault",
        params: FactoryDefaultParams {
            factory_default_mode: mode,
        },
    }
}
