//! Utilities for working with REST-style configuration APIs over HTTP.

use std::marker::PhantomData;

use anyhow::Context;
use log::trace;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

pub struct RequestBuilder<T> {
    path: &'static str,
    data: Value,
    _phantom: PhantomData<T>,
}

impl<T> RequestBuilder<T> {
    pub fn new(path: &'static str) -> Self {
        Self {
            path,
            data: Value::Null,
            _phantom: PhantomData,
        }
    }

    pub fn data(mut self, data: Value) -> Self {
        self.data = data;
        self
    }
}

impl<T> RequestBuilder<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Send,
{
    pub async fn send(self, client: &(impl HttpClient + Sync)) -> Result<T, Error<rest::Error>> {
        let body = serde_json::to_string_pretty(&json!({"data": self.data}))
            .map_err(|e| Error::Request(e.into()))?;
        let request = Request::json(Method::POST, self.path.to_string()).body(body);
        send_request(client, request).await
    }
}
