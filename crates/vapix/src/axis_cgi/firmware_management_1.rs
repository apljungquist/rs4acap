//! The [Firmware Management API].
//!
//! [Firmware Management API]: https://developer.axis.com/vapix/network-video/firmware-management-api/

use serde::{Deserialize, Serialize};

use crate::json_rpc_http::{JsonRpcHttp, JsonRpcHttpLossless};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FactoryDefaultMode {
    None,
    Soft,
    Hard,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FactoryDefaultParams {
    factory_default_mode: FactoryDefaultMode,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FactoryDefaultData {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryDefaultRequest {
    api_version: &'static str,
    method: &'static str,
    params: FactoryDefaultParams,
}

impl FactoryDefaultRequest {
    pub fn new(mode: FactoryDefaultMode) -> Self {
        Self {
            api_version: "1.0",
            method: "factoryDefault",
            params: FactoryDefaultParams {
                factory_default_mode: mode,
            },
        }
    }
}

impl JsonRpcHttp for FactoryDefaultRequest {
    type Data = FactoryDefaultData;
    const PATH: &'static str = "axis-cgi/firmwaremanagement.cgi";
}

impl JsonRpcHttpLossless for FactoryDefaultRequest {
    type Data = FactoryDefaultData;
}
