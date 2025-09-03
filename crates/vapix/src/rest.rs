//! Utilities for working with REST-style configuration APIs.

use std::{
    fmt::{Display, Formatter},
    marker::PhantomData,
};

use anyhow::{anyhow, bail, Context};
use log::debug;
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, value::RawValue, Value};

use crate::Client;

#[derive(Debug, Deserialize)]
struct Response<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(borrow)]
    error: Option<&'a RawValue>,
    #[serde(borrow)]
    status: Option<&'a str>,
}

#[derive(Debug, Deserialize)]
pub struct Error {
    pub code: u16,
    message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let Self { code, message } = self;
        write!(f, "({code}) {message}")
    }
}

impl std::error::Error for Error {}

fn from_response<T>(http_status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let text = text.with_context(|| format!("Could not fetch text, status was {http_status}"))?;
    let Response {
        data,
        status,
        error,
    } = serde_json::from_str(&text)
        .with_context(|| format!("Could not parse response; status: {http_status} text: {text}"))?;
    debug!("Status is {status:?}");
    if let Some(error) = error {
        let error: Error = serde_json::from_str(error.get()).with_context(|| {
            format!(
                "Could not parse error; http-status: {http_status}; config-status: {status:?}; error-text: {}",
                error.get()
            )
        })?;
        return Err(error)
            .with_context(|| format!("http-status: {http_status}; config-status: {status:?};"));
    }
    let Some(data) = data else {
        bail!("Response did not include data, status was {status:?}");
    };
    serde_json::from_str(data.get()).map_err(|e| anyhow!(e))
}

pub struct RequestBuilder<T> {
    client: Client,
    path: &'static str,
    data: Value,
    _phantom: PhantomData<T>,
}

impl<T> RequestBuilder<T>
where
    T: for<'a> Deserialize<'a>,
{
    pub fn new(client: Client, path: &'static str) -> Self {
        Self {
            client,
            path,
            data: Value::Null,
            _phantom: PhantomData,
        }
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }

    pub async fn send(self) -> anyhow::Result<T> {
        let RequestBuilder {
            client,
            path,
            data,
            _phantom,
        } = self;
        let response = client
            .post(path)?
            .json(&json!({"data":data}))
            .send()
            .await?;
        let status = response.status();
        let text = response.text().await;
        from_response(status, text)
    }
}
