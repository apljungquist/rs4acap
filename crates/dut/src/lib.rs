use std::env;

use anyhow::bail;
use url::Host;

fn parse_boolish(s: &str) -> anyhow::Result<bool> {
    match s.to_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        _ => bail!("not a valid bool: {s:?}"),
    }
}

#[derive(Clone, Debug)]
pub struct Device {
    pub host: Host,
    pub username: String,
    pub password: String,
    pub http_port: Option<u16>,
    pub https_port: Option<u16>,
    pub ssh_port: Option<u16>,
    pub https_self_signed: bool,
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
        let https_self_signed = env::var("AXIS_DEVICE_HTTPS_SELF_SIGNED")
            .ok()
            .map(|v| parse_boolish(&v))
            .transpose()?
            .unwrap_or(false);
        Ok(Some(Self {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
            https_self_signed,
        }))
    }

    pub fn to_env(&self) -> Vec<(String, Option<String>)> {
        let Self {
            host,
            username,
            password,
            http_port,
            https_port,
            ssh_port,
            https_self_signed,
        } = self;
        let mut envs = Vec::new();

        envs.push(("AXIS_DEVICE_IP".to_string(), Some(host.to_string())));

        envs.push(("AXIS_DEVICE_USER".to_string(), Some(username.to_string())));

        envs.push(("AXIS_DEVICE_PASS".to_string(), Some(password.to_string())));

        if let Some(p) = ssh_port {
            envs.push(("AXIS_DEVICE_SSH_PORT".to_string(), Some(p.to_string())));
        } else {
            envs.push(("AXIS_DEVICE_SSH_PORT".to_string(), None));
        }

        if let Some(p) = http_port {
            envs.push(("AXIS_DEVICE_HTTP_PORT".to_string(), Some(p.to_string())));
        } else {
            envs.push(("AXIS_DEVICE_HTTP_PORT".to_string(), None));
        }

        if let Some(p) = https_port {
            envs.push(("AXIS_DEVICE_HTTPS_PORT".to_string(), Some(p.to_string())));
        } else {
            envs.push(("AXIS_DEVICE_HTTPS_PORT".to_string(), None));
        }

        envs.push((
            "AXIS_DEVICE_HTTPS_SELF_SIGNED".to_string(),
            Some(if *https_self_signed { "1" } else { "0" }.to_string()),
        ));

        envs
    }

    pub fn clear_env() -> Vec<&'static str> {
        vec!["AXIS_DEVICE_IP"]
    }
}
