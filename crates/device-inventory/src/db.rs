//! Utilities for storing data locally across sessions.
use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context};
use log::debug;
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
