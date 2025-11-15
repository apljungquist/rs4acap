//! Facilities for parsing responses.
use std::{
    fmt::{Display, Formatter},
    net::Ipv4Addr,
    path::PathBuf,
};

use anyhow::Context;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{ser::SerializeSeq, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use url::Host;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirmwareVersion(String);

impl Display for FirmwareVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct DataEnvelope<T> {
    success: bool,
    data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NoDataEnvelope {
    success: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    InvalidJson(anyhow::Error),
    #[error(transparent)]
    SchemaMismatch(anyhow::Error),
    #[error("Not successful")]
    Remote,
}

fn debug_assert_lossless<T>(s: &str, data: &T, meta: Option<Vec<String>>)
where
    T: Clone + for<'de> Deserialize<'de> + Serialize,
{
    let envelope = DataEnvelope {
        success: true,
        data: Some(data.clone()),
        meta,
    };
    let actual = serde_json::to_string(&envelope).unwrap();
    let actual = serde_json::from_str::<Value>(&actual).unwrap();
    let expected = serde_json::from_str::<Value>(s).unwrap();
    debug_assert_eq!(actual, expected);
}

pub fn parse_data<T>(s: &str) -> Result<T, ParseError>
where
    T: Clone + for<'de> Deserialize<'de> + Serialize,
{
    match serde_json::from_str(s) {
        Ok(envelope) => {
            let DataEnvelope::<T> {
                success,
                data,
                meta,
            } = envelope;
            match (success, data) {
                (false, None) => Err(ParseError::Remote),
                (false, Some(_)) => {
                    debug_assert!(false);
                    Err(ParseError::Remote)
                }
                (true, None) => match serde_json::from_str::<T>("null") {
                    Ok(data) => Ok(data),
                    Err(_) => Err(ParseError::SchemaMismatch(anyhow::anyhow!(
                        "Response was a success, but data was missing"
                    ))),
                },
                (true, Some(data)) => {
                    debug_assert_lossless(s, &data, meta);
                    Ok(data)
                }
            }
        }
        Err(e) => match serde_json::from_str::<DataEnvelope<Value>>(s) {
            Ok(_) => Err(ParseError::SchemaMismatch(e.into())),
            Err(e) => Err(ParseError::InvalidJson(e.into())),
        },
    }
}

#[non_exhaustive]
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum DeviceArchitecture {
    Aarch64,
    Armv7hf,
    Armv7l,
    Mips,
}

impl DeviceArchitecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceArchitecture::Aarch64 => "aarch64",
            DeviceArchitecture::Armv7hf => "armv7hf",
            DeviceArchitecture::Armv7l => "armv7l",
            DeviceArchitecture::Mips => "mips",
        }
    }
}

#[non_exhaustive]
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, serde::Deserialize, serde::Serialize,
)]
#[serde(try_from = "u8", into = "u8")]
pub enum DeviceStatus {
    Connected = 1,
    OnLoan = 3,
}

impl DeviceStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeviceStatus::Connected => "connected",
            DeviceStatus::OnLoan => "on-loan",
        }
    }
}

impl TryFrom<u8> for DeviceStatus {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DeviceStatus::Connected),
            3 => Ok(DeviceStatus::OnLoan),
            _ => Err(format!("Unknown device status: {}", value)),
        }
    }
}

impl From<DeviceStatus> for u8 {
    fn from(status: DeviceStatus) -> Self {
        status as u8
    }
}

fn serialize_datetime<S>(v: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let s = v.to_rfc3339_opts(SecondsFormat::Millis, true);
    serializer.serialize_str(&s)
}

fn serialize_datetime_array<S>(vs: &Vec<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(vs.len()))?;
    for v in vs {
        let s = v.to_rfc3339_opts(SecondsFormat::Millis, true);
        seq.serialize_element(&s)?;
    }
    seq.end()
}

fn serialize_semicolon_separated_list<S>(
    list: &[FirmwareVersion],
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let list = list.iter().map(|fw| fw.0.to_string()).collect::<Vec<_>>();
    let s = list.join(";");
    serializer.serialize_str(&s)
}

fn deserialize_semicolon_separated_list<'de, D>(
    deserializer: D,
) -> Result<Vec<FirmwareVersion>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(';')
        .map(|s| FirmwareVersion(s.to_string()))
        .collect())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct ExternalIp(Ipv4Addr);

impl ExternalIp {
    fn port_suffix(&self) -> u16 {
        let external = self.0;
        let [_, _, o2, o3] = external.octets();
        1_000 * o2 as u16 + o3 as u16
    }

    /// Returns the port forwarded to port 80.
    pub fn http_port(&self) -> u16 {
        10_000 + self.port_suffix()
    }

    /// Returns the port forwarded to port 443.
    pub fn https_port(&self) -> u16 {
        40_000 + self.port_suffix()
    }

    /// Returns the port forwarded to port 22.
    pub fn ssh_port(&self) -> u16 {
        20_000 + self.port_suffix()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PortcastDevice {
    pub id: u16,
    pub device_id: u16,
    pub raw_name: String,
    pub model: String,
    pub r#type: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Device {
    pub architecture: DeviceArchitecture,
    #[serde(deserialize_with = "deserialize_semicolon_separated_list")]
    #[serde(serialize_with = "serialize_semicolon_separated_list")]
    pub available_fw_versions: Vec<FirmwareVersion>,
    #[serde(serialize_with = "serialize_datetime_array")]
    pub booked: Vec<DateTime<Utc>>,
    /// Despite it's name, the device is not accessible from the internet at this IP.
    /// But it can be used to infer connect options such as ports.
    /// However, it is best to refrain from connecting to these devices since they may be in use.
    pub external_ip: ExternalIp,
    pub firmware_version: FirmwareVersion,
    pub id: LoanableId,
    pub image_url: PathBuf,
    pub model: String,
    pub platform: String,
    pub portcast: bool,
    portcast_device: Option<PortcastDevice>,
    pub release_year: u16,
    pub resolution: Option<String>,
    pub sdcard: bool,
    pub status: DeviceStatus,
    pub r#type: String,
}

impl Device {
    pub fn host(&self) -> Host {
        Host::Ipv4(Ipv4Addr::from([195, 60, 68, 14]))
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewLoanable {
    pub id: LoanableId,
    pub internal_ip: String,
    pub model: String,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct NewLoan {
    #[serde(serialize_with = "serialize_datetime")]
    pub loan_start: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub loan_end: DateTime<Utc>,
    pub id: LoanId,
    pub selected_firmware: FirmwareVersion,
    pub password: String,
    #[serde(serialize_with = "serialize_datetime")]
    pub started_at: DateTime<Utc>,
    pub status: DeviceStatus,
    pub username: String,
    pub loanable: NewLoanable,
    pub meta: Vec<String>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loan {
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
    pub id: LoanId,
    #[serde(serialize_with = "serialize_datetime")]
    pub loan_end: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub loan_start: DateTime<Utc>,
    pub loanable: Loanable,
    pub meta: Vec<String>,
    pub password: String,
    pub selected_firmware: FirmwareVersion,
    #[serde(serialize_with = "serialize_datetime")]
    pub started_at: DateTime<Utc>,
    pub status: DeviceStatus,
    pub username: String,
}

impl Loan {
    /// Returns a host accessible from the internet with ports forwarded to the device.
    pub fn host(&self) -> Host {
        Host::Ipv4(Ipv4Addr::from([195, 60, 68, 14]))
    }

    fn base_port(&self) -> anyhow::Result<u16> {
        let (_, port) = self
            .loanable
            .internal_ip
            .split_once(':')
            .context("Internal IP has no port")?;
        let port: u16 = port
            .parse()
            .context("Internal IP port is not a valid port number")?;
        Ok(port)
    }

    /// Returns the port forwarded to port 80.
    pub fn http_port(&self) -> u16 {
        let from_suffix = self.loanable.external_ip.http_port();
        if cfg!(debug_assertions) {
            let from_base_port = self.base_port().unwrap();
            debug_assert_eq!(from_base_port, from_suffix);
        }
        from_suffix
    }

    /// Returns the port forwarded to port 443.
    pub fn https_port(&self) -> u16 {
        self.loanable.external_ip.https_port()
    }

    /// Returns the port forwarded to port 22.
    pub fn ssh_port(&self) -> u16 {
        self.loanable.external_ip.ssh_port()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct LoanId(u32);

impl Display for LoanId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loanable {
    /// Despite it's name, the device is not accessible from the internet at this IP.
    /// Consider using [`Loan::host`] instead.
    pub external_ip: ExternalIp,
    pub internal_ip: String,
    pub id: LoanableId,
    pub model: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct LoanableId(u16);

impl LoanableId {
    pub fn as_u16(&self) -> u16 {
        self.0
    }
}

impl Display for LoanableId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
