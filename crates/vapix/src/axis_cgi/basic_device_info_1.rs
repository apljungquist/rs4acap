//! The [Basic device information] API.
//!
//! [Basic device information]: https://developer.axis.com/vapix/network-video/basic-device-information/

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{client::Client, json_rpc};

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

pub struct BasicDeviceInfo1 {
    client: Client,
}

impl BasicDeviceInfo1 {
    pub fn get_all_unrestricted_properties(
        self,
    ) -> json_rpc::RequestBuilder<AllUnrestrictedPropertiesData> {
        json_rpc::RequestBuilder {
            client: self.client,
            path: "axis-cgi/basicdeviceinfo.cgi",
            json: json!({
                "method": "getAllUnrestrictedProperties",
                "apiVersion": "1.0",
            }),
            _phantom: Default::default(),
        }
    }
}

impl Client {
    pub fn basic_device_info_1(&self) -> BasicDeviceInfo1 {
        BasicDeviceInfo1 {
            client: self.clone(),
        }
    }
}
