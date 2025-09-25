//! Utilities for working with JSON RPC style APIs.

use std::time::Duration;

use anyhow::Context;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{client_blocking::BlockingClient, json_rpc::parse_data, json_rpc_http::JsonRpcHttp};

fn from_response<T>(status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let text = text.with_context(|| format!("Could not fetch text, status was {status}"))?;
    parse_data(&text).with_context(|| format!("Could not parse data, status was {status}"))
}

// These can probably be factored in a nicer way,
// but it shows how little code is needed to add support for virtually all JSON RPC style apis.
pub trait BlockingJsonRpcHttp: Serialize + Send + Sized {
    type Data: for<'a> Deserialize<'a>;

    const PATH: &'static str;

    fn send_with_timeout(
        self,
        client: &BlockingClient,
        timeout: Duration,
    ) -> anyhow::Result<Self::Data> {
        let response = client
            .post(Self::PATH)?
            .json(&self)
            .timeout(timeout)
            .send()?;
        let status = response.status();
        let text = response.text();
        from_response(status, text)
    }
}

impl<T> BlockingJsonRpcHttp for T
where
    T: JsonRpcHttp,
{
    type Data = T::Data;
    const PATH: &'static str = T::PATH;
}
