//! Utilities for working with REST-style configuration APIs over HTTP.

use std::{future::Future, marker::PhantomData};

use anyhow::Context;
use log::trace;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{rest::parse_data, Client};

fn from_response<T>(http_status: StatusCode, text: reqwest::Result<String>) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a>,
{
    let text = text.with_context(|| format!("Could not fetch text, status was {http_status}"))?;
    if cfg!(debug_assertions) {
        println!("Received {http_status}: {text}");
    }
    parse_data(&text)
        .with_context(|| format!("Could not parse response as data; status: {http_status}."))
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

impl<T> RestHttp for RequestBuilder<T>
where
    T: for<'a> Deserialize<'a> + Send,
{
    type RequestData = Value;
    type ResponseData = T;
    const METHOD: Method = Method::POST;

    fn to_path_and_data(self) -> anyhow::Result<(String, Self::RequestData)> {
        let Self {
            path,
            data,
            _phantom,
        } = self;
        Ok((path.to_string(), data))
    }
}

pub trait RestHttp: Send + Sized {
    type RequestData: Send + Serialize;
    type ResponseData: for<'a> Deserialize<'a>;

    const METHOD: Method;

    fn to_path_and_data(self) -> anyhow::Result<(String, Self::RequestData)>;

    fn send(
        self,
        client: &Client,
    ) -> impl Future<Output = anyhow::Result<Self::ResponseData>> + Send {
        async move {
            let (path, data) = self.to_path_and_data()?;
            let json = json!({"data":data});
            if cfg!(debug_assertions) {
                println!(
                    "Sending to {path}: {}",
                    serde_json::to_string(&json).unwrap()
                );
            }
            let response = client
                .request(Self::METHOD, &path)?
                .json(&json)
                .send()
                .await?;
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
