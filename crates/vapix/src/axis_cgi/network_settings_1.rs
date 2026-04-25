//! The [Network Settings API].
//!
//! [Network Settings API]: https://developer.axis.com/vapix/network-video/network-settings-api/

use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient},
    json_rpc, json_rpc_http,
};

const PATH: &str = "axis-cgi/network_settings.cgi";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetGlobalProxyConfigurationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    http_proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    https_proxy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    no_proxy: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetGlobalProxyConfigurationBody {
    api_version: &'static str,
    method: &'static str,
    params: SetGlobalProxyConfigurationParams,
}

#[derive(Debug)]
pub struct SetGlobalProxyConfigurationRequest {
    params: SetGlobalProxyConfigurationParams,
}

impl Default for SetGlobalProxyConfigurationRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl SetGlobalProxyConfigurationRequest {
    pub fn new() -> Self {
        Self {
            params: SetGlobalProxyConfigurationParams {
                http_proxy: None,
                https_proxy: None,
                no_proxy: None,
            },
        }
    }

    pub fn http_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.params.http_proxy = Some(proxy.into());
        self
    }

    pub fn https_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.params.https_proxy = Some(proxy.into());
        self
    }

    pub fn no_proxy(mut self, no_proxy: impl Into<String>) -> Self {
        self.params.no_proxy = Some(no_proxy.into());
        self
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<SetGlobalProxyConfigurationData, Error<json_rpc::Error>> {
        let body = SetGlobalProxyConfigurationBody {
            api_version: "1.0",
            method: "setGlobalProxyConfiguration",
            params: self.params,
        };
        json_rpc_http::send_request(client, PATH, &body).await
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
pub struct SetGlobalProxyConfigurationData {}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct GetNetworkInfoBody {
    api_version: &'static str,
    method: &'static str,
}

#[derive(Debug)]
pub struct GetNetworkInfoRequest;

impl Default for GetNetworkInfoRequest {
    fn default() -> Self {
        Self::new()
    }
}

impl GetNetworkInfoRequest {
    pub fn new() -> Self {
        Self
    }

    pub async fn send(
        self,
        client: &(impl HttpClient + Sync),
    ) -> Result<NetworkInfoData, Error<json_rpc::Error>> {
        let body = GetNetworkInfoBody {
            api_version: "1.0",
            method: "getNetworkInfo",
        };
        json_rpc_http::send_request(client, PATH, &body).await
    }
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfoData {
    pub system: SystemInfo,
    pub devices: Vec<DeviceInfo>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub tcp_ecn_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_supported_vlans: Option<u32>,
    pub device_switching: DeviceSwitching,
    pub hostname: Hostname,
    pub resolver: Resolver,
    /// Absent on AXIS OS < 11.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_proxies: Option<GlobalProxies>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSwitching {
    pub mode: String,
    pub manual_active_devices: Vec<String>,
    pub devices: Vec<String>,
    pub active_devices: Vec<String>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hostname {
    pub use_dhcp_hostname: bool,
    pub hostname: String,
    pub static_hostname: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolver {
    pub use_dhcp_resolver_info: bool,
    pub name_servers: Vec<String>,
    pub static_name_servers: Vec<String>,
    pub max_supported_static_name_servers: u32,
    pub search_domains: Vec<String>,
    pub static_search_domains: Vec<String>,
    pub max_supported_static_search_domains: u32,
    pub domain_name: String,
    pub static_domain_name: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalProxies {
    pub http_proxy: String,
    pub https_proxy: String,
    pub no_proxy: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub mac_address: String,
    pub part_of_bridge: String,
    pub link: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_state: Option<String>,
    pub wired: WiredInfo,
    #[serde(rename = "IPv4")]
    pub ipv4: Ipv4Info,
    #[serde(rename = "IPv6")]
    pub ipv6: Ipv6Info,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WiredInfo {
    pub link_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_link_modes: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_downlink: Option<bool>,
    #[serde(rename = "8021X")]
    pub dot1x: Dot1xInfo,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dot1xInfo {
    pub enabled: bool,
    pub status: String,
    pub mode: String,
    pub configurations: Vec<Dot1xConfiguration>,
    pub supported_modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "MACsecSecured")]
    pub macsec_secured: Option<bool>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dot1xConfiguration {
    pub mode: String,
    pub params: Dot1xParams,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dot1xParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "is_password_set")]
    pub is_password_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eapol_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peap_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_client: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "certsCA")]
    pub certs_ca: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "is_mka_cak_set")]
    pub is_mka_cak_set: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mka_ckn: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv4Info {
    pub enabled: bool,
    pub configuration_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_local_mode: Option<String>,
    pub addresses: Vec<Ipv4Address>,
    pub max_supported_static_address_configurations: u32,
    pub static_address_configurations: Vec<StaticAddressConfiguration>,
    pub default_router: String,
    pub static_default_router: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "useStaticDHCPFallback"
    )]
    pub use_static_dhcp_fallback: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "useDHCPStaticRoutes"
    )]
    pub use_dhcp_static_routes: Option<bool>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv4Address {
    pub address: String,
    pub prefix_length: u32,
    pub origin: String,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub broadcast: Option<String>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaticAddressConfiguration {
    pub address: String,
    pub prefix_length: u32,
    pub broadcast: String,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6Info {
    pub enabled: bool,
    pub addresses: Vec<Ipv6Address>,
}

#[non_exhaustive]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ipv6Address {
    pub address: String,
    pub prefix_length: u32,
    pub origin: String,
    pub scope: String,
}
