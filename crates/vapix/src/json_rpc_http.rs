//! Utilities for working with JSON RPC style APIs.

use std::future::Future;

use anyhow::Context;
use log::trace;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{json_rpc::parse_data, Client};

fn from_response<T>(status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let text = text.with_context(|| format!("Could not fetch text, status was {status}"))?;
    parse_data(&text).with_context(|| format!("Could not parse data, status was {status}"))
}

pub trait JsonRpcHttp: Serialize + Send + Sized {
    type Data: for<'a> Deserialize<'a>;

    const PATH: &'static str;

    fn send(self, client: &Client) -> impl Future<Output = anyhow::Result<Self::Data>> + Send {
        async move {
            let response = client.post(Self::PATH)?.json(&self).send().await?;
            let status = response.status();
            let text = response.text().await;

            if cfg!(debug_assertions) {
                if let Ok(text) = text.as_deref() {
                    trace!("Received {status}: {text}");
                }
            }

            from_response(status, text)
        }
    }
}
