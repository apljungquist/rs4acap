//! Utilities for storing data locally across sessions.
use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context};
use log::debug;
use rs4a_authentication::{CookieStore, SessionCookie};
use url::Host;

use crate::psst::Password;

const DEVICES_FILE_NAME: &str = "devices.json";

fn default_true() -> bool {
    true
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Device {
    pub host: Host,
    pub username: String,
    pub password: Password,
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub ssh_port: Option<u16>,
    pub model: Option<String>,
    #[serde(default = "default_true")]
    pub https_self_signed: bool,
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
            https_self_signed,
        } = value;
        Self {
            host,
            username,
            password: Password::new(password),
            http_port,
            https_port,
            ssh_port,
            model: None,
            https_self_signed,
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
            https_self_signed,
        } = value;
        rs4a_dut::Device {
            host,
            username,
            password: password.dangerous_reveal().to_string(),
            http_port,
            https_port,
            ssh_port,
            https_self_signed,
        }
    }
}

pub struct Database {
    dir: PathBuf,
    cookie_store: CookieStore,
}

impl Database {
    pub fn open_or_create(data_dir: Option<PathBuf>) -> anyhow::Result<Self> {
        let (db_dir, cookie_store) = match data_dir {
            None => {
                let dir = dirs::data_dir()
                    .context("Could not infer a data directory")?
                    .join("rs4a-device-inventory");
                (dir, CookieStore::open_default()?)
            }
            Some(custom) => {
                let cookie_store = CookieStore::new(custom.clone());
                (custom, cookie_store)
            }
        };
        // TODO: Consider limiting where we can write to
        fs::create_dir_all(&db_dir).context("Failed to create the data directory")?;
        Ok(Self {
            dir: db_dir,
            cookie_store,
        })
    }

    pub(crate) fn read_cookie(&self) -> anyhow::Result<Option<SessionCookie>> {
        self.cookie_store.read()
    }

    pub fn write_cookie(&self, cookie: &SessionCookie) -> anyhow::Result<()> {
        self.cookie_store.write(cookie)
    }

    pub fn read_devices(&self) -> anyhow::Result<HashMap<String, Device>> {
        let file = self.dir.join(DEVICES_FILE_NAME);
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
        fs::write(self.dir.join(DEVICES_FILE_NAME), devices)
            .context("Failed to write devices")
            .map(|_| ())
    }
}
