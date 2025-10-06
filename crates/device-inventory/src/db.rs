//! Utilities for storing data locally across sessions.
use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context};
use log::debug;
use rs4a_vlt::responses::LoanId;
use url::Host;

use crate::psst::Password;

const COOKIE_FILE_NAME: &str = "vlt-cookie";
const DEVICES_FILE_NAME: &str = "devices.json";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Device {
    pub host: Host,
    pub username: String,
    pub password: Password,
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub ssh_port: Option<u16>,
    pub model: Option<String>,
    pub loan_id: Option<LoanId>,
}

impl Device {
    /// No two distinct devices have the same fingerprint at the same time.
    pub fn fingerprint(&self) -> (Host, u16) {
        (self.host.clone(), self.http_port.unwrap_or(80))
    }
}

impl From<rs4a_dut::Device> for Device {
    fn from(value: rs4a_dut::Device) -> Self {
        let rs4a_dut::Device {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
        } = value;
        Self {
            host,
            username,
            password: Password::new(password),
            http_port,
            https_port,
            ssh_port,
            model: None,
            loan_id: None,
        }
    }
}

impl From<Device> for rs4a_dut::Device {
    fn from(value: Device) -> Self {
        let Device {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
            model: _,
            loan_id: _,
        } = value;
        rs4a_dut::Device {
            host,
            username,
            password: password.dangerous_reveal().to_string(),
            http_port,
            https_port,
            ssh_port,
        }
    }
}

pub struct Database(PathBuf);

impl Database {
    pub fn open_or_create(data_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let db_dir = match data_dir {
            None => dirs::data_dir()
                .context("Could not infer a data directory")?
                .join("rs4a-device-inventory"),
            Some(custom) => custom,
        };
        // TODO: Consider limiting where we can write to
        fs::create_dir_all(&db_dir).context("Failed to create the data directory")?;
        Ok(Self(db_dir))
    }

    pub(crate) fn read_cookie(&self) -> anyhow::Result<Option<String>> {
        match fs::read_to_string(self.0.join(COOKIE_FILE_NAME)) {
            Ok(t) => Ok(Some(t)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("{COOKIE_FILE_NAME} not found, returning None");
                Ok(None)
            }
            Err(e) => Err(anyhow!(e)),
        }
    }

    pub fn write_cookie(&self, content: &str) -> anyhow::Result<()> {
        fs::write(self.0.join(COOKIE_FILE_NAME), content.trim())
            .context("Failed to write cookie")
            .map(|_| ())
    }

    pub fn read_devices(&self) -> anyhow::Result<HashMap<String, Device>> {
        let file = self.0.join(DEVICES_FILE_NAME);
        match fs::read_to_string(&file) {
            Ok(t) => serde_json::from_str(&t)
                .context("Failed to deserialize devices")
                .with_context(|| format!("Consider removing {file:?}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("{DEVICES_FILE_NAME} not found, returning an empty collection");
                Ok(HashMap::new())
            }
            Err(e) => Err(anyhow!(e)),
        }
    }

    pub fn write_devices(&self, devices: &HashMap<String, Device>) -> anyhow::Result<()> {
        let devices =
            serde_json::to_string_pretty(devices).context("Failed to serialize devices")?;
        fs::write(self.0.join(DEVICES_FILE_NAME), devices)
            .context("Failed to write devices")
            .map(|_| ())
    }
}
