//! The [Parameter Management] API.
//!
//! [Parameter Management]: https://developer.axis.com/vapix/network-video/parameter-management/

use std::{collections::HashMap, fmt::Debug};

use anyhow::{bail, Context};
use reqwest::StatusCode;

use crate::Client;

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
