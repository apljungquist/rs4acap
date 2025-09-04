//! The [Basic device information] API.
//!
//! [Basic device information]: https://developer.axis.com/vapix/network-video/basic-device-information/

use serde::{Deserialize, Serialize};

use crate::json_rpc_http::JsonRpcHttp;

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AllUnrestrictedPropertiesData {
    pub property_list: UnrestrictedProperties,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UnrestrictedProperties {
    pub brand: String,
    pub build_date: String,
    #[serde(rename = "HardwareID")]
    pub hardware_id: String,
    pub prod_full_name: String,
    pub prod_nbr: String,
    pub prod_short_name: String,
    pub prod_type: String,
    pub prod_variant: String,
    pub serial_number: String,
    pub version: String,
    #[serde(rename = "WebURL")]
    pub web_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAllUnrestrictedPropertiesRequest {
    api_version: &'static str,
    method: &'static str,
}

impl Default for GetAllUnrestrictedPropertiesRequest {
    fn default() -> Self {
        Self {
            api_version: "1.0",
            method: "getAllUnrestrictedProperties",
        }
    }
}

impl JsonRpcHttp for GetAllUnrestrictedPropertiesRequest {
    type Data = AllUnrestrictedPropertiesData;
    const PATH: &'static str = "axis-cgi/basicdeviceinfo.cgi";
}

pub fn get_all_unrestricted_properties() -> GetAllUnrestrictedPropertiesRequest {
    GetAllUnrestrictedPropertiesRequest::default()
}
