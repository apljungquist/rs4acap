use std::{env, fs, path::PathBuf};

use anyhow::{anyhow, Context};
use url::Host;

const FILENAME: &str = "dut-v0.json";

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Device {
    pub host: Host,
    pub username: String,
    pub password: String,
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub ssh_port: Option<u16>,
}

impl Device {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Some(host) = env::var_os("AXIS_DEVICE_IP") else {
            return Ok(None);
        };
        let host = Host::parse(host.to_string_lossy().as_ref())?;
        let username = env::var("AXIS_DEVICE_USER")?;
        let password = env::var("AXIS_DEVICE_PASS")?;
        let http_port = env::var("AXIS_DEVICE_HTTP_PORT")
            .ok()
            .map(|p| p.parse())
            .transpose()?;
        let https_port = env::var("AXIS_DEVICE_HTTPS_PORT")
            .ok()
            .map(|p| p.parse())
            .transpose()?;
        let ssh_port = env::var("AXIS_DEVICE_SSH_PORT")
            .ok()
            .map(|p| p.parse())
            .transpose()?;
        Ok(Some(Self {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
        }))
    }

    fn dir() -> anyhow::Result<PathBuf> {
        Ok(dirs::data_dir()
            .context("Could not infer a data directory")?
            .join("rs4a-dut"))
    }

    pub fn from_fs() -> anyhow::Result<Option<Self>> {
        let file = Self::dir()?.join(FILENAME);
        match fs::read_to_string(&file) {
            Ok(t) => serde_json::from_str(&t)
                .context("Failed to deserialize device")
                .with_context(|| format!("Consider removing {file:?}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(anyhow!(e)),
        }
    }

    pub fn to_env(&self) -> Vec<(String, String)> {
        let Self {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
        } = self;
        let mut envs = Vec::new();

        envs.push(("AXIS_DEVICE_IP".to_string(), host.to_string()));
        envs.push(("AXIS_DEVICE_USER".to_string(), username.to_string()));
        envs.push(("AXIS_DEVICE_PASS".to_string(), password.to_string()));
        if let Some(p) = ssh_port {
            envs.push(("AXIS_DEVICE_SSH_PORT".to_string(), p.to_string()));
        }
        if let Some(p) = http_port {
            envs.push(("AXIS_DEVICE_HTTP_PORT".to_string(), p.to_string()));
        }
        if let Some(p) = https_port {
            envs.push(("AXIS_DEVICE_HTTPS_PORT".to_string(), p.to_string()));
        }
        envs.push(("AXIS_DEVICE_HTTPS_SELF_SIGNED".to_string(), "1".to_string()));

        envs
    }

    pub fn to_fs(&self) -> anyhow::Result<PathBuf> {
        // TODO: Consider looking in current working directory, etc.
        let device = serde_json::to_string_pretty(&self).context("Failed to serialize device")?;
        let dir = Self::dir()?;
        fs::create_dir_all(&dir).context("Failed to create the data directory")?;
        let destination = dir.join(FILENAME);
        match fs::write(&destination, device) {
            Ok(_) => Ok(destination),
            Err(e) => Err(anyhow!(e)),
        }
    }
}
