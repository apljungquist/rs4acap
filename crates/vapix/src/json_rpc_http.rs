//! Utilities for working with JSON RPC style APIs over HTTP.

use anyhow::Context;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient, Request},
    json_rpc,
    json_rpc::parse_data_lossless,
};

pub fn from_response<T>(
    status: StatusCode,
    text: reqwest::Result<String>,
) -> Result<T, Error<json_rpc::Error>>
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    let text = text
        .with_context(|| format!("Could not fetch text, status was {status}"))
        .map_err(Error::Transport)?;
    Error::flat_result(parse_data_lossless(&text))
}

pub async fn send_request<Req, Resp>(
    client: &(impl HttpClient + Sync),
    path: &str,
    request: &Req,
) -> Result<Resp, Error<json_rpc::Error>>
where
    Req: Serialize,
    Resp: for<'a> Deserialize<'a> + Serialize,
{
    let body = serde_json::to_string_pretty(request).map_err(|e| Error::Request(e.into()))?;
    let request = Request::new(Method::POST, path.to_string()).json(body);
    let response = client.execute(request).await.map_err(Error::Transport)?;
    from_response(response.status, response.body)
}
