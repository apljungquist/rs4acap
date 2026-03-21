//! The parameter management CGI at `axis-cgi/param.cgi`.

use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use anyhow::{bail, Context};
use reqwest::StatusCode;

use crate::Client;

const PATH: &str = "axis-cgi/param.cgi";

pub struct UpdateRequest {
    parameters: HashMap<String, String>,
}

impl UpdateRequest {
    /// Add a parameter to be updated.
    ///
    /// # Panics
    ///
    /// Panics if the same parameter is set twice or if `parameter` is a reserved word
    /// (`action` or `usergroup`).
    pub fn set<P: Debug + Display, V: Display>(mut self, parameter: P, value: V) -> Self {
        let parameter = parameter.to_string();
        assert_ne!(parameter, "action");
        assert_ne!(parameter, "usergroup");
        assert!(
            self.parameters
                .insert(parameter, value.to_string())
                .is_none(),
            "Expected each parameter at most once"
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

pub fn update() -> UpdateRequest {
    UpdateRequest {
        parameters: HashMap::new(),
    }
}
