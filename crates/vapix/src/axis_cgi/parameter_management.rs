//! The [Parameter Management] API.
//!
//! [Parameter Management]: https://developer.axis.com/vapix/network-video/parameter-management/

use std::{collections::HashMap, fmt, fmt::Debug};

use anyhow::{bail, Context};
use reqwest::{Method, StatusCode};

use crate::{
    http::{HttpClient, Request},
    Client,
};

const PATH: &str = "axis-cgi/param.cgi";

fn bool2str(b: bool) -> &'static str {
    match b {
        true => "yes",
        false => "no",
    }
}

#[derive(Clone, Debug, Default)]
pub struct UpdateRequest {
    parameters: HashMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Resolution {
    pub width_px: u32,
    pub height_px: u32,
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}", self.width_px, self.height_px)
    }
}

impl std::str::FromStr for Resolution {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (w, h) = s
            .split_once('x')
            .context("expected format '{width}x{height}'")?;
        Ok(Self {
            width_px: w.parse().context("invalid width")?,
            height_px: h.parse().context("invalid height")?,
        })
    }
}

/// A typed parameter that knows how to request and parse its value.
pub trait Parameter {
    type Value;
    const KEY: &'static str;
    fn parse(raw: &str) -> anyhow::Result<Self::Value>;
}

/// [`Parameter`] for `Properties.Image.Resolution`.
///
/// These are the available pixel resolutions for image sources.
pub struct ImageResolution;

impl Parameter for ImageResolution {
    type Value = Vec<Resolution>;
    const KEY: &'static str = "Properties.Image.Resolution";

    fn parse(raw: &str) -> anyhow::Result<Vec<Resolution>> {
        raw.split(',').map(|s| s.trim().parse()).collect()
    }
}

/// The response from a parameter list request.
#[derive(Clone, Debug)]
pub struct ParamList(HashMap<String, String>);

impl ParamList {
    /// Get the string value of a parameter by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|s| s.as_str())
    }

    /// Parse a typed parameter key from the response.
    pub fn parse<K: Parameter>(&self) -> anyhow::Result<Option<K::Value>> {
        self.0.get(K::KEY).map(|v| K::parse(v)).transpose()
    }
}

#[derive(Clone, Debug)]
pub struct ListRequest {
    group: String,
}

impl ListRequest {
    // TODO: Add support for retrieving groups of parameters.
    pub fn new<T: Parameter>() -> Self {
        Self {
            group: T::KEY.to_string(),
        }
    }

    pub async fn send(self, client: &impl HttpClient) -> anyhow::Result<ParamList> {
        let path = format!("{PATH}?action=list&group={}", self.group);
        let response = client
            .execute(Request::new(Method::GET, path))
            .await
            .context("sending param.cgi request")?;

        let text = response.body.context("reading param.cgi response")?;

        if let Some(e) = text.trim().strip_prefix("# Error: ") {
            bail!("{e}");
        }

        let mut params = HashMap::new();
        for line in text.lines() {
            if let Some((k, v)) = line.split_once('=') {
                params.insert(k.to_string(), v.to_string());
            }
        }
        Ok(ParamList(params))
    }
}

impl UpdateRequest {
    pub fn network_ssh_enabled(mut self, value: bool) -> Self {
        self.parameters.insert(
            "root.Network.SSH.Enabled".to_string(),
            bool2str(value).to_string(),
        );
        self
    }

    pub async fn send(self, client: &Client) -> anyhow::Result<()> {
        let mut query: Vec<(&str, &str)> = vec![("action", "update")];
        for (k, v) in &self.parameters {
            query.push((k, v));
        }
        let response = client.get(PATH)?.query(&query).send().await?;
        let status = response.status();
        let text = response
            .text()
            .await
            .with_context(|| format!("status code: {status}"))?;

        if status == StatusCode::OK && text.trim() == "OK" {
            Ok(())
        } else if let Some(e) = text.trim().strip_prefix("# Error: ") {
            bail!("{e}")
        } else {
            bail!("Unexpected response: {status} {text}")
        }
    }
}
