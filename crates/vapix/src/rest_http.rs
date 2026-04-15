//! Utilities for working with REST-style configuration APIs over HTTP.

use anyhow::Context;
use log::trace;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    http::{Error, HttpClient, Request},
    rest,
    rest::parse_data_lossless,
};

// The device configuration API reports status codes both in the HTTP header and in the body.
// TODO: Consider if there is any value in this.

pub(crate) fn from_response<T>(
    http_status: StatusCode,
    text: reqwest::Result<String>,
) -> Result<T, Error<rest::Error>>
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    let text = text
        .with_context(|| format!("Could not fetch text, status was {http_status}"))
        .map_err(Error::Transport)?;
    if cfg!(debug_assertions) {
        trace!("Received {http_status}: {text}");
    }
    Error::flat_result(parse_data_lossless(&text))
}

pub(crate) async fn send_request<T>(
    client: &(impl HttpClient + Sync),
    request: Request,
) -> Result<T, Error<rest::Error>>
where
    T: for<'a> Deserialize<'a> + Serialize,
{
    let response = client.execute(request).await.map_err(Error::Transport)?;
    from_response(response.status, response.body)
}
