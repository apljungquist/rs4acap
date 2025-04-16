//! Utilities for working with JSON RPC style APIs.
use std::marker::PhantomData;

use anyhow::{anyhow, bail, Context};
use log::debug;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::Client;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Response<'a> {
    #[serde(borrow)]
    data: Option<&'a RawValue>,
    #[serde(borrow)]
    error: Option<&'a RawValue>,
}

// TODO: Improve error handling
fn from_response<T>(status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let text = text.with_context(|| format!("Could not fetch text, status was {status}"))?;
    let Response { data, error } = serde_json::from_str(&text)
        .with_context(|| format!("Could not parse response, status was {status}"))?;
    let data = match (data, error) {
        (Some(d), Some(e)) => {
            debug!("data: {d:?}, error: {e:?}");
            bail!("Response included data and an error ({e:?})")
        }
        (Some(d), None) => d,
        (None, Some(e)) => bail!("Response included an error ({e:?})"),
        (None, None) => bail!("Response included neither data nor error"),
    };
    serde_json::from_str(data.get()).map_err(|e| anyhow!(e))
}

pub struct RequestBuilder<T> {
    pub(crate) client: Client,
    pub(crate) path: &'static str,
    pub(crate) json: serde_json::Value,
    pub(crate) _phantom: PhantomData<T>,
}

impl<T> RequestBuilder<T>
where
    T: for<'a> Deserialize<'a>,
{
    pub async fn send(self) -> anyhow::Result<T> {
        let RequestBuilder {
            client,
            path,
            json,
            _phantom,
        } = self;
        let response = client.post(path)?.json(&json).send().await?;
        let status = response.status();
        let text = response.text().await;
        from_response(status, text)
    }
}
